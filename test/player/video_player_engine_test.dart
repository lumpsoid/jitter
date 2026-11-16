import 'dart:async';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/video_player_engine.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:video_player_platform_interface/video_player_platform_interface.dart';

/// The initialized event a player sends once its media has loaded.
VideoEvent _initialized({Duration duration = const Duration(minutes: 1)}) =>
    VideoEvent(
      eventType: VideoEventType.initialized,
      duration: duration,
      size: const Size(1920, 1080),
    );

/// A platform that records the calls a controller makes and plays the events
/// a test pushes, so the engine runs with no player behind it.
class _FakePlatform extends VideoPlayerPlatform {
  /// Each call the controller made, as a short string: `create 1`,
  /// `seekTo 10`, `setVolume 0.8`, `play`, `pause`, `dispose 1`.
  final List<String> calls = [];

  /// The volume of each `setVolume` call, in order.
  final List<double> volumes = [];

  final Map<int, StreamController<VideoEvent>> _events = {};
  final Map<int, Duration> _positions = {};
  int _lastId = 0;

  double? get lastVolume => volumes.isEmpty ? null : volumes.last;

  /// Plays [event] on the last player created, as the platform would.
  void push(VideoEvent event) {
    _events[_lastId]?.add(event);
  }

  /// Finishes the last player's load, as the platform would once its media
  /// has loaded.
  void finishInitialize() => push(_initialized());

  /// Fails the last player's load with a platform error, as the platform
  /// would once its media is found to be unplayable.
  void failLast() {
    _events[_lastId]?.addError(
      PlatformException(code: 'init', message: 'no stream'),
    );
  }

  @override
  Future<void> init() async {}

  /// Whether the next player fails to be created, before it has an id.
  bool failCreate = false;

  @override
  Future<int?> createWithOptions(VideoCreationOptions options) async {
    if (failCreate) {
      throw PlatformException(code: 'create', message: 'no player');
    }
    _lastId += 1;
    calls.add('create $_lastId');
    return _lastId;
  }

  /// The initialized event is queued before the controller listens, so
  /// `initialize()` completes once it does.
  /// Whether the next player fails to initialize, with a platform error.
  bool failInitialize = false;

  /// Whether the players hold their load: no initialized event is sent, so
  /// `initialize()` waits until [finishInitialize].
  bool holdInitialize = false;

  /// The length the next player reports once its media has loaded.
  Duration nextDuration = const Duration(minutes: 1);

  @override
  Stream<VideoEvent> videoEventsFor(int playerId) {
    final events = StreamController<VideoEvent>();
    if (failInitialize) {
      events.addError(PlatformException(code: 'init', message: 'no stream'));
    } else if (!holdInitialize) {
      events.add(_initialized(duration: nextDuration));
    }
    _events[playerId] = events;
    return events.stream;
  }

  @override
  Future<void> setLooping(int playerId, bool looping) async {}

  @override
  Future<void> setPlaybackSpeed(int playerId, double speed) async {}

  /// Whether [play] waits for this to complete, so a load sits inside it.
  Completer<void>? holdPlay;

  /// Whether [setVolume] waits for this to complete, so a load sits inside it.
  Completer<void>? holdSetVolume;

  @override
  Future<void> play(int playerId) async {
    calls.add('play');
    await holdPlay?.future;
  }

  @override
  Future<void> pause(int playerId) async {
    calls.add('pause');
  }

  @override
  Future<void> setVolume(int playerId, double volume) async {
    calls.add('setVolume $volume');
    volumes.add(volume);
    await holdSetVolume?.future;
  }

  @override
  Future<void> seekTo(int playerId, Duration position) async {
    calls.add('seekTo ${position.inSeconds}');
    _positions[playerId] = position;
  }

  @override
  Future<Duration> getPosition(int playerId) async =>
      _positions[playerId] ?? Duration.zero;

  @override
  Future<void> dispose(int playerId) async {
    calls.add('dispose $playerId');
    await _events.remove(playerId)?.close();
  }
}

/// Lets a load run to its end, and the events the fake queued reach the
/// controller and the streams.
Future<void> _settle() => pumpEventQueue();

/// Opens [url] and lets the load finish, so the player is playing.
Future<void> _open(
  VideoPlayerEngine engine,
  String url, {
  Duration start = Duration.zero,
}) async {
  await engine.open(url, start: start);
  await _settle();
}

List<bool> _bufferings(List<PlayerBuffering> sent) =>
    sent.map((signal) => signal.buffering).toList();

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  const url = 'https://x/v.m3u8';
  late _FakePlatform fake;
  late List<PlayerBuffering> sent;
  late List<String> logged;
  late VideoPlayerEngine engine;

  setUp(() {
    fake = _FakePlatform();
    VideoPlayerPlatform.instance = fake;
    sent = [];
    logged = [];
    engine = VideoPlayerEngine(sendBuffering: sent.add, log: logged.add);
  });

  test("open plays from the start at the user's volume", () async {
    await _open(engine, url, start: const Duration(seconds: 10));

    expect(
      fake.calls,
      containsAllInOrder(['create 1', 'seekTo 10', 'setVolume 1.0', 'play']),
    );
    expect(engine.isPlaying, isTrue);

    await engine.stop();
  });

  test('a second open disposes the first player', () async {
    await _open(engine, url);
    await _open(engine, 'https://x/other.m3u8');

    expect(
      fake.calls,
      containsAllInOrder(['create 1', 'dispose 1', 'create 2']),
    );

    await engine.stop();
  });

  test("a fade plays under the user's volume; open and stop end it", () async {
    await _open(engine, url);
    await engine.setVolume(80);
    expect(fake.lastVolume, closeTo(0.8, 1e-9));

    await engine.setFade(0.5);
    expect(fake.lastVolume, closeTo(0.4, 1e-9));
    expect(engine.volumes.now(), 80);

    await _open(engine, url);
    expect(fake.lastVolume, closeTo(0.8, 1e-9));

    await engine.setFade(0.5);
    await engine.stop();
    await _open(engine, url);
    expect(fake.lastVolume, closeTo(0.8, 1e-9));

    await engine.stop();
  });

  test('seekBy stays within the media', () async {
    await _open(engine, url, start: const Duration(seconds: 10));

    await engine.seekBy(const Duration(seconds: -20));
    await engine.seekBy(const Duration(minutes: 5));

    expect(
      fake.calls,
      containsAllInOrder(['seekTo 10', 'seekTo 0', 'seekTo 60']),
    );

    await engine.stop();
  });

  test('a buffering change is sent once each way', () async {
    await _open(engine, url);

    fake.push(VideoEvent(eventType: VideoEventType.bufferingStart));
    await _settle();
    expect(_bufferings(sent), [true]);

    // Still buffering, but the value changes: the report must not repeat.
    fake.push(
      VideoEvent(
        eventType: VideoEventType.bufferingUpdate,
        buffered: [DurationRange(Duration.zero, const Duration(seconds: 5))],
      ),
    );
    await _settle();
    expect(engine.buffers.now(), const Duration(seconds: 5));
    expect(_bufferings(sent), [true]);

    fake.push(VideoEvent(eventType: VideoEventType.bufferingEnd));
    await _settle();
    expect(_bufferings(sent), [true, false]);

    await engine.stop();
  });

  test('playing passes through false on stop', () async {
    final states = <bool>[];
    final subscription = engine.playing.listen(states.add);

    await _open(engine, url);
    await engine.stop();
    await _settle();
    await subscription.cancel();

    expect(states, containsAllInOrder([true, false]));
    expect(states.last, isFalse);
  });

  test('stepFrame steps a paused picture, not a playing one', () async {
    final stepped = <int>[];
    engine = VideoPlayerEngine(
      sendBuffering: sent.add,
      log: logged.add,
      stepFrames: (_, frames) async {
        stepped.add(frames);
      },
    );

    await _open(engine, url);
    await engine.stepFrame(forward: true);
    expect(stepped, isEmpty);

    await engine.pause();
    await engine.stepFrame(forward: false);
    expect(stepped, [-1]);

    await engine.stop();
  });

  test('a stop ends a buffering wait', () async {
    await _open(engine, url);
    fake.push(VideoEvent(eventType: VideoEventType.bufferingStart));
    await _settle();

    await engine.stop();

    expect(_bufferings(sent), [true, false]);
  });

  test('a second open ends a buffering wait', () async {
    await _open(engine, url);
    fake.push(VideoEvent(eventType: VideoEventType.bufferingStart));
    await _settle();

    await _open(engine, url);

    expect(_bufferings(sent), [true, false]);
    await engine.stop();
  });

  test('a failed open still ends a fade', () async {
    await _open(engine, url);
    await engine.setVolume(80);
    await engine.setFade(0.5);

    fake.failInitialize = true;
    await engine.open(url, start: Duration.zero);
    await _settle();

    expect(logged, hasLength(1));
    expect(logged.single, contains('no stream'));
    expect(engine.volumes.now(), 80);

    fake.failInitialize = false;
    await _open(engine, url);
    expect(fake.lastVolume, closeTo(0.8, 1e-9));

    await engine.stop();
  });

  test('open returns before the player is ready', () async {
    fake.holdInitialize = true;
    await engine.open(url, start: Duration.zero);
    await _settle();

    expect(fake.calls, isNot(contains('play')));
    expect(engine.isPlaying, isFalse);

    fake.finishInitialize();
    await _settle();

    expect(fake.calls, contains('play'));
    expect(engine.isPlaying, isTrue);

    await engine.stop();
  });

  test('a stop during a load drops it', () async {
    fake.holdInitialize = true;
    await engine.open(url, start: Duration.zero);
    await _settle();

    await engine.stop();
    fake.finishInitialize();
    await _settle();

    expect(fake.calls, isNot(contains('play')));
    expect(fake.calls, contains('dispose 1'));
    expect(engine.isPlaying, isFalse);
  });

  test('stopForExit during a load drops it', () async {
    fake.holdInitialize = true;
    await engine.open(url, start: Duration.zero);
    await _settle();

    await engine.stopForExit();
    fake.finishInitialize();
    await _settle();

    expect(fake.calls, isNot(contains('play')));
    expect(fake.calls, contains('dispose 1'));
    expect(engine.isPlaying, isFalse);
  });

  test('a second open during a load plays only the second', () async {
    fake.holdInitialize = true;
    await engine.open(url, start: Duration.zero);
    await _settle();

    fake.holdInitialize = false;
    await _open(engine, 'https://x/other.m3u8');

    expect(fake.calls, containsAllInOrder(['create 2', 'play']));
    expect(fake.calls.where((call) => call == 'play'), hasLength(1));
    expect(fake.calls, contains('dispose 1'));
    expect(engine.isPlaying, isTrue);

    await engine.stop();
  });

  test(
    'a player that cannot be created is logged and holds up nothing',
    () async {
      fake.failCreate = true;
      await engine.open(url, start: Duration.zero);
      await _settle();

      expect(logged, hasLength(1));
      expect(logged.single, contains('no player'));

      fake.failCreate = false;
      await engine
          .open(url, start: Duration.zero)
          .timeout(const Duration(seconds: 2));
      await _settle();

      expect(engine.isPlaying, isTrue);

      await engine.stop();
    },
  );

  test('a superseded load that fails is not logged', () async {
    fake.holdInitialize = true;
    await engine.open(url, start: Duration.zero);
    await _settle();

    await engine.stop();
    fake.failLast();
    await _settle();

    expect(logged, isEmpty);
  });

  test('a stop while the player starts drops it', () async {
    final states = <bool>[];
    final subscription = engine.playing.listen(states.add);
    fake.holdPlay = Completer<void>();

    await engine.open(url, start: Duration.zero);
    await _settle();
    await engine.stop();

    // Stop does not wait on the held play: it pauses and disposes the
    // published controller at once.
    expect(fake.calls, containsAllInOrder(['play', 'pause', 'dispose 1']));
    expect(engine.isPlaying, isFalse);

    fake.holdPlay!.complete();
    await _settle();
    await subscription.cancel();

    expect(engine.isPlaying, isFalse);
    expect(states.last, isFalse);
  });

  test('a seek during a load starts there', () async {
    fake.holdInitialize = true;
    await engine.open(url, start: const Duration(seconds: 10));
    await _settle();

    await engine.seek(const Duration(seconds: 30));
    fake.finishInitialize();
    await _settle();

    expect(fake.calls, contains('seekTo 30'));
    expect(fake.calls, isNot(contains('seekTo 10')));
    expect(engine.isPlaying, isTrue);

    await engine.stop();
  });

  test('a relative seek during a load keeps the start', () async {
    fake.holdInitialize = true;
    await engine.open(url, start: const Duration(seconds: 10));
    await _settle();

    await engine.seekBy(const Duration(seconds: 5));
    fake.finishInitialize();
    await _settle();

    expect(fake.calls, contains('seekTo 10'));

    await engine.stop();
  });

  test('a fade set while the player starts is applied', () async {
    fake.holdPlay = Completer<void>();
    await engine.setVolume(80);
    await engine.open(url, start: Duration.zero);
    await _settle();

    // Set during the held play: 80 at a fade of 0.5 is 40, so 0.4.
    await engine.setFade(0.5);
    expect(fake.lastVolume, closeTo(0.4, 1e-9));

    fake.holdPlay!.complete();
    await _settle();

    expect(fake.lastVolume, closeTo(0.4, 1e-9));
    expect(engine.volumes.now(), 80);

    await engine.stop();
  });

  test('a stop while the volume is set drops it before play', () async {
    fake.holdSetVolume = Completer<void>();

    await engine.open(url, start: Duration.zero);
    await _settle();
    await engine.stop();
    fake.holdSetVolume!.complete();
    await _settle();

    expect(fake.calls, isNot(contains('play')));
    expect(fake.calls, contains('dispose 1'));
    expect(engine.isPlaying, isFalse);
  });

  test('a live stream, whose length is unbounded, reports no length', () async {
    // mdk reports a live stream's length as INT64_MAX microseconds.
    fake.nextDuration = const Duration(microseconds: 9223372036854775807);
    await _open(engine, url);

    expect(engine.duration, Duration.zero);
    expect(engine.durations.now(), Duration.zero);

    await engine.stop();
  });
}
