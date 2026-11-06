import 'package:jitter/src/bindings/bindings.dart';
import 'package:media_kit/media_kit.dart';

/// Reads, from mpv, what this run's log keeps for a bug report: mpv's
/// versions, how it plays each stream, and the frames it lost. Rust writes
/// the lines; nothing here decides.
class MpvProbes {
  new(this._mpv);

  final NativePlayer _mpv;

  /// A stream has loaded since the last [ended].
  bool _loaded = false;

  Future<void> versions() async => MpvVersions(
    mpv: await _read('mpv-version'),
    ffmpeg: await _read('ffmpeg-version'),
  ).sendSignalToRust();

  /// The video's [params] changed: the first time they carry a pixel
  /// format after a load, mpv has decoded a frame, so reports how it plays
  /// it. Not on the first size: before a frame, mpv gives the container's
  /// size alone, and has picked no decoder yet.
  Future<void> videoParams(VideoParams params) async {
    if (_loaded || params.pixelformat == null) return;
    _loaded = true;
    PlaybackProbe(
      hwdec: await _read('hwdec-current'),
      hwdecInterop: await _read('hwdec-interop'),
      vo: await _read('current-vo'),
      gpuContext: await _read('current-gpu-context'),
      videoCodec: await _read('video-format'),
      width: await _read('video-params/w'),
      height: await _read('video-params/h'),
      fps: await _read('container-fps'),
      pixelformat: await _read('video-params/pixelformat'),
      hwPixelformat: await _read('video-params/hw-pixelformat'),
      audioCodec: await _read('audio-codec-name'),
      ao: await _read('current-ao'),
    ).sendSignalToRust();
  }

  /// Reports the frames lost over the stream that loaded, before it stops
  /// or another replaces it.
  Future<void> ended() async {
    if (!_loaded) return;
    _loaded = false;
    PlaybackDrops(
      output: await _read('frame-drop-count'),
      decoder: await _read('decoder-frame-drop-count'),
      mistimed: await _read('mistimed-frame-count'),
    ).sendSignalToRust();
  }

  /// [property]'s text; empty where mpv has none or the read failed, which
  /// Rust logs as `unknown`.
  Future<String> _read(String property) async {
    try {
      return await _mpv.getProperty(property);
    } on Object catch (_) {
      return '';
    }
  }
}
