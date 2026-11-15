import 'package:fvp/fvp.dart' as fvp;
import 'package:jitter/player/media_kit_engine.dart';
import 'package:jitter/player/player_engine.dart';
import 'package:logging/logging.dart';
import 'package:material_ui/material_ui.dart';
import 'package:media_kit/media_kit.dart';
import 'package:media_kit_video/media_kit_video.dart';
import 'package:video_player/video_player.dart';

import 'fvp_engine.dart';

/// One engine under test: its player, its picture and its own counters.
abstract interface class BenchSubject {
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
/// reports. Its `BENCH_MPV_HWDEC` line goes to the `log` callback that
/// [benchSubject] was given.
class MpvSubject implements BenchSubject {
  new(this._engine, this._hwdec, this._log);

  final MediaKitEngine _engine;
  final String? _hwdec;
  final void Function(String line) _log;
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
    _log('BENCH_MPV_HWDEC $hwdec');
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

/// A video_player engine (fvp's or ExoPlayer's), with its picture. The mpv
/// counters do not apply, so its end line is the end text it was given
/// alone, which says where the decoder is reported.
class VideoPlayerSubject implements BenchSubject {
  new(this._engine, this._name, this._end);

  final FvpEngine _engine;
  final String _name;
  final String _end;

  @override
  String get name => _name;

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
  Future<String> endWindow() async => _end;
}

/// The engine [name] runs as. fvp registers itself for Windows, Linux and
/// Android with [decoders] in place of its own list when given, and forwards
/// what libmdk logs through [log]. exo is the same [FvpEngine] left
/// unregistered, so video_player plays it with ExoPlayer. mpv needs media_kit's
/// setup first, decodes with [hwdec] when given, and reports its `hwdec` line
/// through [log].
BenchSubject benchSubject(
  String name, {
  required void Function(String line) log,
  List<String>? decoders,
  String? hwdec,
}) {
  if (name == 'exo') {
    return VideoPlayerSubject(FvpEngine(), 'exo', ' hwdec=see-logcat');
  }
  if (name == 'fvp') {
    Logger.root.level = Level.INFO;
    Logger.root.onRecord.listen(
      (record) => log(
        'MDK ${record.level.name} ${record.loggerName}: ${record.message}',
      ),
    );
    final options = <String, Object>{
      'platforms': ['windows', 'linux', 'android'],
    };
    if (decoders != null) options['video.decoders'] = decoders;
    fvp.registerWith(options: options);
    return VideoPlayerSubject(FvpEngine(), 'fvp', ' hwdec=see-MDK-log');
  }
  MediaKit.ensureInitialized();
  return MpvSubject(MediaKitEngine(), hwdec, log);
}
