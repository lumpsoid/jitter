import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/scheduler.dart';
import 'package:fvp/fvp.dart' as fvp;
import 'package:jitter/player/media_kit_engine.dart';
import 'package:jitter/player/player_engine.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:logging/logging.dart';
import 'package:media_kit/media_kit.dart';
import 'package:media_kit_video/media_kit_video.dart';
import 'package:rinf/rinf.dart';
import 'package:video_player/video_player.dart';

import 'fvp_engine.dart';

/// Plays `JITTER_BENCH_URL` full-window with the engine named by
/// `JITTER_BENCH_ENGINE` (`mpv` or `fvp`), as a bare picture with no
/// controls, then reports the frames drawn over the measured window.
///
/// Both engines run in this one build, so the system load is comparable.
/// Output: `BENCH_START engine=<name>` when measuring begins, then one
/// `BENCH_END ...` line when it ends. Failures are `BENCH_ERROR` on stderr.
Future<void> main() async {
  final env = Platform.environment;
  final url = env['JITTER_BENCH_URL'];
  if (url == null || url.isEmpty) _fail(2, 'no JITTER_BENCH_URL');
  final name = env['JITTER_BENCH_ENGINE'] ?? 'mpv';
  if (name != 'mpv' && name != 'fvp') {
    _fail(2, 'unknown JITTER_BENCH_ENGINE $name');
  }
  final warmup = _seconds(env, 'JITTER_BENCH_WARMUP', 15);
  final seconds = _seconds(env, 'JITTER_BENCH_SECONDS', 60);

  WidgetsFlutterBinding.ensureInitialized();
  await initializeRust(assignRustSignal);
  final frames = _Frames();
  SchedulerBinding.instance.addTimingsCallback(frames.onTimings);
  final subject = _subject(name);
  runApp(
    MaterialApp(
      debugShowCheckedModeBanner: false,
      home: ColoredBox(
        color: Colors.black,
        child: Center(child: subject.picture),
      ),
    ),
  );
  unawaited(
    _measure(
      subject,
      frames,
      url: url,
      warmup: Duration(seconds: warmup),
      window: Duration(seconds: seconds),
    ),
  );
}

/// Reports a usage error on stderr and exits with [code].
Never _fail(int code, String message) {
  stderr.writeln('BENCH_ERROR $message');
  exit(code);
}

/// The whole number of seconds in the variable [name], or [fallback] when
/// it is unset.
int _seconds(Map<String, String> env, String name, int fallback) {
  final raw = env[name];
  if (raw == null) return fallback;
  return int.tryParse(raw) ?? _fail(2, 'bad $name $raw');
}

/// The engine [name] runs as. fvp registers itself for Linux and forwards
/// what libmdk logs; mpv needs media_kit's setup first.
_Subject _subject(String name) {
  if (name == 'fvp') {
    Logger.root.level = Level.INFO;
    Logger.root.onRecord.listen(
      (record) => stdout.writeln(
        'MDK ${record.level.name} ${record.loggerName}: ${record.message}',
      ),
    );
    fvp.registerWith(
      options: {
        'platforms': ['linux'],
      },
    );
    return _FvpSubject(FvpEngine());
  }
  MediaKit.ensureInitialized();
  return _MpvSubject(MediaKitEngine());
}

/// Opens [url] at zero and lets it warm up, then times the [window] while
/// counting frames, and ends the run.
Future<void> _measure(
  _Subject subject,
  _Frames frames, {
  required String url,
  required Duration warmup,
  required Duration window,
}) async {
  try {
    await subject.engine.open(url, start: Duration.zero);
    await Future<void>.delayed(warmup);
    await subject.startWindow();
    frames.measuring = true;
    final clock = Stopwatch()..start();
    stdout.writeln('BENCH_START engine=${subject.name}');
    await Future<void>.delayed(window);
    frames.measuring = false;
    clock.stop();
    final fields = await subject.endWindow();
    final seconds = clock.elapsedMilliseconds / 1000;
    final fps = frames.count / seconds;
    stdout.writeln(
      'BENCH_END engine=${subject.name}'
      ' seconds=${seconds.toStringAsFixed(1)}'
      ' frames=${frames.count}'
      ' fps=${fps.toStringAsFixed(1)}'
      ' position_s=${subject.engine.position.inSeconds}$fields',
    );
    await subject.engine.stop();
    exit(0);
  } on Object catch (error) {
    stderr.writeln('BENCH_ERROR $error');
    exit(1);
  }
}

/// Counts the raster frames Flutter draws while [measuring].
class _Frames {
  bool measuring = false;
  int count = 0;

  void onTimings(List<FrameTiming> timings) {
    if (measuring) count += timings.length;
  }
}

/// One engine under test: its player, its picture and its own counters.
abstract interface class _Subject {
  String get name;

  PlayerEngine get engine;

  Widget get picture;

  /// Reads the counters at the start of the measured window.
  Future<void> startWindow();

  /// The end line's fields after the measured window, each with a leading
  /// space.
  Future<String> endWindow();
}

/// The mpv engine, with its picture and the mpv counters the end line
/// reports.
class _MpvSubject implements _Subject {
  new(this._engine);

  final MediaKitEngine _engine;
  int? _frameDrops;
  int? _decoderDrops;

  @override
  String get name => 'mpv';

  @override
  PlayerEngine get engine => _engine;

  @override
  Widget get picture => Video(
    controller: _engine.controller,
    // NoVideoControls is null; the analyzer types the constant as dynamic.
    controls: null,
  );

  @override
  Future<void> startWindow() async {
    _frameDrops = await _int('frame-drop-count');
    _decoderDrops = await _int('decoder-frame-drop-count');
  }

  @override
  Future<String> endWindow() async {
    final hwdec = await _property('hwdec-current') ?? '?';
    final vo = await _property('current-vo') ?? '?';
    final voDrops = _delta(await _int('frame-drop-count'), _frameDrops);
    final decoderDrops = _delta(
      await _int('decoder-frame-drop-count'),
      _decoderDrops,
    );
    return ' hwdec=$hwdec vo=$vo vo_drops=$voDrops'
        ' decoder_drops=$decoderDrops';
  }

  /// [end] less [start], or `?` when either was unreadable.
  static String _delta(int? end, int? start) =>
      end == null || start == null ? '?' : '${end - start}';

  Future<int?> _int(String property) async =>
      int.tryParse(await _property(property) ?? '');

  /// The mpv property's value, or `null` when it is unset or empty.
  Future<String?> _property(String property) async {
    final platform = _engine.controller.player.platform;
    if (platform is! NativePlayer) return null;
    final value = await platform.getProperty(property);
    return value.isEmpty ? null : value;
  }
}

/// The fvp engine, with its picture. The mpv counters do not apply, so its
/// end line points at the MDK log for the decoder.
class _FvpSubject implements _Subject {
  new(this._engine);

  final FvpEngine _engine;

  @override
  String get name => 'fvp';

  @override
  PlayerEngine get engine => _engine;

  @override
  Widget get picture => ValueListenableBuilder<VideoPlayerController?>(
    valueListenable: _engine.controller,
    builder: (_, controller, _) => controller == null
        ? const SizedBox.shrink()
        : AspectRatio(
            aspectRatio: controller.value.aspectRatio,
            child: VideoPlayer(controller),
          ),
  );

  @override
  Future<void> startWindow() async {}

  @override
  Future<String> endWindow() async => ' hwdec=see-MDK-log';
}
