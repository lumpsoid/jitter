import 'package:jitter/src/bindings/bindings.dart';
import 'package:media_kit/media_kit.dart';

/// Reads, from mpv, what this run's log keeps for a bug report: mpv's
/// versions, how it plays each stream, and the frames it lost. Rust writes
/// the lines; nothing here decides.
class MpvProbes {
  new(this._mpv);

  final NativePlayer _mpv;

  /// mpv has decoded a frame of the stream that loaded, since [ended].
  bool _decoded = false;

  /// That stream's [PlaybackProbe] is sent.
  bool _reported = false;

  Future<void> versions() async => MpvVersions(
    mpv: await _read('mpv-version'),
    ffmpeg: await _read('ffmpeg-version'),
  ).sendSignalToRust();

  /// Reports how mpv plays a stream once it also draws it: on Android the
  /// first frame decodes before the video's surface exists, on `vo=null`,
  /// and the output starts only when the surface arrives.
  Future<void> watchOutput() =>
      _mpv.observeProperty('current-vo', (_) => _playback());

  /// The video's [params] changed: the first time they carry a pixel
  /// format after a load, mpv has decoded a frame, so reports how it plays
  /// it. Not on the first size: before a frame, mpv gives the container's
  /// size alone, and has picked no decoder yet.
  Future<void> videoParams(VideoParams params) async {
    if (params.pixelformat == null) return;
    _decoded = true;
    await _playback();
  }

  /// Sends the [PlaybackProbe], once per stream, when mpv has decoded a
  /// frame and has an output to draw it on. Switching the output resets
  /// the decoder, which clears the video's params until its next frame:
  /// then the next [videoParams] reports.
  Future<void> _playback() async {
    if (!_decoded || _reported) return;
    final vo = await _read('current-vo');
    final pixelformat = await _read('video-params/pixelformat');
    if (!_decoded || _reported || vo == 'null' || pixelformat.isEmpty) return;
    _reported = true;
    PlaybackProbe(
      hwdec: await _read('hwdec-current'),
      hwdecInterop: await _read('hwdec-interop'),
      vo: vo,
      gpuContext: await _read('current-gpu-context'),
      videoCodec: await _read('video-format'),
      width: await _read('video-params/w'),
      height: await _read('video-params/h'),
      fps: await _read('container-fps'),
      pixelformat: pixelformat,
      hwPixelformat: await _read('video-params/hw-pixelformat'),
      audioCodec: await _read('audio-codec-name'),
      ao: await _read('current-ao'),
    ).sendSignalToRust();
  }

  /// Reports the frames lost over the stream that loaded, before it stops
  /// or another replaces it.
  Future<void> ended() async {
    if (!_decoded) return;
    _decoded = false;
    _reported = false;
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
