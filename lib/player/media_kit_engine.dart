import 'package:jitter/player/player_engine.dart';
import 'package:media_kit/media_kit.dart';
import 'package:media_kit_video/media_kit_video.dart';

/// [PlayerEngine] on `media_kit` (libmpv). One for the app's lifetime; the
/// player page shows [controller]. The player is created on first use, so
/// building this needs no libmpv.
class MediaKitEngine implements PlayerEngine {
  late final Player _player = Player();

  /// What the player page's `Video` renders.
  late final VideoController controller = VideoController(_player);

  @override
  Future<void> open(String url, {required Duration start}) =>
      _player.open(Media(url, start: start));

  @override
  Future<void> stop() => _player.stop();

  @override
  Duration get position => _player.state.position;

  @override
  Duration get duration => _player.state.duration;
}
