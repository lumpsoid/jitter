import 'dart:async';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/video_player_engine.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:video_player_platform_interface/video_player_platform_interface.dart';

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

  @override
  Future<void> init() async {}

  @override
  Future<int?> createWithOptions(VideoCreationOptions options) async {
    _lastId += 1;
    calls.add('create $_lastId');
    return _lastId;
  }

  /// The initialized event is queued before the controller listens, so
  /// `initialize()` completes once it does.
  /// Whether the next player fails to initialize, with a platform error.
  bool failInitialize = false;

  @override
  Stream<VideoEvent> videoEventsFor(int playerId) {
    final events = StreamController<VideoEvent>();
    if (failInitialize) {
      events.addError(PlatformException(code: 'init', message: 'no stream'));
    } else {
      events.add(
        VideoEvent(
          eventType: VideoEventType.initialized,
          duration: const Duration(minutes: 1),
          size: const Size(1920, 1080),
        ),
      );
    }
    _events[playerId] = events;
    return events.stream;
  }

  @override
  Future<void> setLooping(int playerId, bool looping) async {}

  @override
  Future<void> setPlaybackSpeed(int playerId, double speed) async {}

  @override
  Future<void> play(int playerId) async {
    calls.add('play');
  }

  @override
  Future<void> pause(int playerId) async {
    calls.add('pause');
  }

  @override
  Future<void> setVolume(int playerId, double volume) async {
    calls.add('setVolume $volume');
    volumes.add(volume);
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

/// Lets the events the fake queued reach the controller and the streams.
Future<void> _settle() => Future<void>.delayed(Duration.zero);

List<bool> _bufferings(List<PlayerBuffering> sent) =>
    sent.map((signal) => signal.buffering).toList();

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  const url = 'https://x/v.m3u8';
  late _FakePlatform fake;
  late List<PlayerBuffering> sent;
  late VideoPlayerEngine engine;

  setUp(() {
    fake = _FakePlatform();
    VideoPlayerPlatform.instance = fake;
    sent = [];
    engine = VideoPlayerEngine(sendBuffering: sent.add);
  });

  test("open plays from the start at the user's volume", () async {
    await engine.open(url, start: const Duration(seconds: 10));

    expect(
      fake.calls,
      containsAllInOrder(['create 1', 'seekTo 10', 'setVolume 1.0', 'play']),
    );
    expect(engine.isPlaying, isTrue);

    await engine.stop();
  });

  test('a second open disposes the first player', () async {
    await engine.open(url, start: Duration.zero);
    await engine.open('https://x/other.m3u8', start: Duration.zero);

    expect(
      fake.calls,
      containsAllInOrder(['create 1', 'dispose 1', 'create 2']),
    );

    await engine.stop();
  });

  test("a fade plays under the user's volume; open and stop end it", () async {
    await engine.open(url, start: Duration.zero);
    await engine.setVolume(80);
    expect(fake.lastVolume, closeTo(0.8, 1e-9));

    await engine.setFade(0.5);
    expect(fake.lastVolume, closeTo(0.4, 1e-9));
    expect(engine.volumes.now(), 80);

    await engine.open(url, start: Duration.zero);
    expect(fake.lastVolume, closeTo(0.8, 1e-9));

    await engine.setFade(0.5);
    await engine.stop();
    await engine.open(url, start: Duration.zero);
    expect(fake.lastVolume, closeTo(0.8, 1e-9));

    await engine.stop();
  });

  test('seekBy stays within the media', () async {
    await engine.open(url, start: const Duration(seconds: 10));

    await engine.seekBy(const Duration(seconds: -20));
    await engine.seekBy(const Duration(minutes: 5));

    expect(
      fake.calls,
      containsAllInOrder(['seekTo 10', 'seekTo 0', 'seekTo 60']),
    );

    await engine.stop();
  });

  test('a buffering change is sent once each way', () async {
    await engine.open(url, start: Duration.zero);

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

    await engine.open(url, start: Duration.zero);
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
      stepFrames: (_, frames) async {
        stepped.add(frames);
      },
    );

    await engine.open(url, start: Duration.zero);
    await engine.stepFrame(forward: true);
    expect(stepped, isEmpty);

    await engine.pause();
    await engine.stepFrame(forward: false);
    expect(stepped, [-1]);

    await engine.stop();
  });

  test('a stop ends a buffering wait', () async {
    await engine.open(url, start: Duration.zero);
    fake.push(VideoEvent(eventType: VideoEventType.bufferingStart));
    await _settle();

    await engine.stop();

    expect(_bufferings(sent), [true, false]);
  });

  test('a second open ends a buffering wait', () async {
    await engine.open(url, start: Duration.zero);
    fake.push(VideoEvent(eventType: VideoEventType.bufferingStart));
    await _settle();

    await engine.open(url, start: Duration.zero);

    expect(_bufferings(sent), [true, false]);
    await engine.stop();
  });

  test('a failed open still ends a fade', () async {
    await engine.open(url, start: Duration.zero);
    await engine.setVolume(80);
    await engine.setFade(0.5);

    fake.failInitialize = true;
    await expectLater(
      engine.open(url, start: Duration.zero),
      throwsA(isA<PlatformException>()),
    );
    expect(engine.volumes.now(), 80);

    fake.failInitialize = false;
    await engine.open(url, start: Duration.zero);
    expect(fake.lastVolume, closeTo(0.8, 1e-9));

    await engine.stop();
  });
}
