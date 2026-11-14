import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
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
/// controls, then reports what the engine did over the measured window.
///
/// Both engines run in this one build, so the system load is comparable.
/// Output: `BENCH_START engine=<name>` when measuring begins, then one
/// `BENCH_END ...` line when it ends. Failures are `BENCH_ERROR` on stderr.
///
/// `JITTER_BENCH_MPV_HWDEC`, when set, is the mpv `hwdec` the mpv engine
/// decodes with. It is set before the URL opens and echoed as
/// `BENCH_MPV_HWDEC <value>`.
///
/// Flutter frame timings are not counted: the Linux embedder redraws video
/// textures without a Dart frame, so they never show up as `FrameTiming`s.
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
  final subject = _subject(name, _decoders(env), _mpvHwdec(env));
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

/// The decoder names in `JITTER_BENCH_FVP_DECODERS` (comma-separated), or
/// `null` when it is unset or empty, which keeps fvp's own list.
List<String>? _decoders(Map<String, String> env) {
  final names = (env['JITTER_BENCH_FVP_DECODERS'] ?? '')
      .split(',')
      .map((name) => name.trim())
      .where((name) => name.isNotEmpty)
      .toList();
  return names.isEmpty ? null : names;
}

/// The mpv `hwdec` in `JITTER_BENCH_MPV_HWDEC`, or `null` when it is unset
/// or empty, which keeps the player's own choice.
String? _mpvHwdec(Map<String, String> env) {
  final value = env['JITTER_BENCH_MPV_HWDEC'];
  return value == null || value.isEmpty ? null : value;
}

/// The engine [name] runs as. fvp registers itself for Linux with
/// [decoders] in place of its own list when given, and forwards what libmdk
/// logs; mpv needs media_kit's setup first, and decodes with [hwdec] when
/// given.
_Subject _subject(String name, List<String>? decoders, String? hwdec) {
  if (name == 'fvp') {
    Logger.root.level = Level.INFO;
    Logger.root.onRecord.listen(
      (record) => stdout.writeln(
        'MDK ${record.level.name} ${record.loggerName}: ${record.message}',
      ),
    );
    final options = <String, Object>{
      'platforms': ['linux'],
    };
    if (decoders != null) options['video.decoders'] = decoders;
    fvp.registerWith(options: options);
    return _FvpSubject(FvpEngine());
  }
  MediaKit.ensureInitialized();
  return _MpvSubject(MediaKitEngine(), hwdec);
}

/// Opens [url] at zero and lets it warm up, then times the [window] and
/// ends the run.
Future<void> _measure(
  _Subject subject, {
  required String url,
  required Duration warmup,
  required Duration window,
}) async {
  try {
    await subject.prepare();
    await subject.engine.open(url, start: Duration.zero);
    await Future<void>.delayed(warmup);
    await subject.startWindow();
    final clock = Stopwatch()..start();
    stdout.writeln('BENCH_START engine=${subject.name}');
    await Future<void>.delayed(window);
    clock.stop();
    final fields = await subject.endWindow();
    final seconds = clock.elapsedMilliseconds / 1000;
    stdout.writeln(
      'BENCH_END engine=${subject.name}'
      ' seconds=${seconds.toStringAsFixed(1)}'
      ' position_s=${subject.engine.position.inSeconds}$fields',
    );
    await subject.engine.stop();
    exit(0);
  } on Object catch (error) {
    stderr.writeln('BENCH_ERROR $error');
    exit(1);
  }
}

/// One engine under test: its player, its picture and its own counters.
abstract interface class _Subject {
  String get name;

  PlayerEngine get engine;

  Widget get picture;

  /// Readies the engine before the URL opens.
  Future<void> prepare();

  /// Reads the counters at the start of the measured window.
  Future<void> startWindow();

  /// The end line's fields after the measured window, each with a leading
  /// space.
  Future<String> endWindow();
}

/// The mpv engine, with its picture and the mpv counters the end line
/// reports.
class _MpvSubject implements _Subject {
  new(this._engine, this._hwdec);

  final MediaKitEngine _engine;
  final String? _hwdec;
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

  /// Sets mpv's `hwdec` when one is given. The video controller sets its own
  /// `hwdec` as it is created, so this waits for that first and then is the
  /// last word before the URL opens.
  @override
  Future<void> prepare() async {
    final hwdec = _hwdec;
    if (hwdec == null) return;
    await _engine.controller.platform.future;
    final platform = _engine.controller.player.platform;
    if (platform is! NativePlayer) {
      throw StateError('the mpv player has no libmpv handle');
    }
    await platform.setProperty('hwdec', hwdec);
    stdout.writeln('BENCH_MPV_HWDEC $hwdec');
  }

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
    final vfFps = double.tryParse(await _property('estimated-vf-fps') ?? '');
    return ' hwdec=$hwdec vo=$vo vo_drops=$voDrops'
        ' decoder_drops=$decoderDrops'
        ' vf_fps=${vfFps?.toStringAsFixed(1) ?? '?'}';
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
  Future<void> prepare() async {}

  @override
  Future<void> startWindow() async {}

  @override
  Future<String> endWindow() async => ' hwdec=see-MDK-log';
}
