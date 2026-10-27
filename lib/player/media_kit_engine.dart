import 'dart:math' as math;

import 'package:jitter/player/faded_volume.dart';
import 'package:jitter/player/player_engine.dart';
import 'package:jitter/player/video_zoom.dart';
import 'package:media_kit/media_kit.dart';
import 'package:media_kit_video/media_kit_video.dart';

/// [PlayerEngine] on `media_kit` (libmpv). One for the app's lifetime; the
/// player page shows [controller]. The player is created on first use, so
/// building this needs no libmpv.
class MediaKitEngine implements PlayerEngine {
  late final Player _player = Player();

  /// What the player page's `Video` renders.
  late final VideoController controller = VideoController(_player);

  final _volume = FadedVolume();

  /// Ends a fade first, so the new media plays at the user's volume.
  @override
  Future<void> open(String url, {required Duration start}) async {
    await _setVolume(_volume.end());
    await _player.open(Media(url, start: start));
  }

  /// Stops, then ends a fade: silent until stopped, the user's volume
  /// after.
  @override
  Future<void> stop() async {
    await _player.stop();
    await _setVolume(_volume.end());
  }

  @override
  Future<void> setFade(double level) =>
      _setVolume(_volume.fade(level, current: _player.state.volume));

  Future<void> togglePlaying() => _player.playOrPause();

  Future<void> play() => _player.play();

  Future<void> pause() => _player.pause();

  /// Whether it is playing now, then each change; an `open` passes
  /// through `false`.
  Stream<bool> get playing async* {
    yield _player.state.playing;
    yield* _player.stream.playing;
  }

  /// Mutes, or unmutes back to the volume it was muted at; mid-fade, under
  /// the fade.
  Future<void> toggleMute() =>
      _player.setVolume(_volume.toggleMute(current: _player.state.volume));

  /// Shows the picture zoomed by [zoom]. libmpv draws it, so media_kit's
  /// controls stay unzoomed over it; it holds across `open`s until reset.
  Future<void> setZoom(VideoZoom zoom) async {
    if (_player.platform case final NativePlayer mpv) {
      // `video-zoom` is the scale's base-2 log; `video-pan-*` shifts the
      // picture by fractions of its zoomed size.
      await mpv.setProperty('video-zoom', '${math.log(zoom.scale) / math.ln2}');
      await mpv.setProperty('video-pan-x', '${0.5 - zoom.center.dx}');
      await mpv.setProperty('video-pan-y', '${0.5 - zoom.center.dy}');
    }
  }

  Future<void> _setVolume(double? volume) async {
    if (volume != null) await _player.setVolume(volume);
  }

  @override
  Duration get position => _player.state.position;

  @override
  Duration get duration => _player.state.duration;
}
