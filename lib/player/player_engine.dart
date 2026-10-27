/// The video player a `PlayerCommand` drives: the border a test replaces
/// with a fake. `MediaKitEngine` is the real one.
abstract interface class PlayerEngine {
  /// Plays [url] from [start].
  Future<void> open(String url, {required Duration start});

  Future<void> stop();

  Duration get position;

  /// Zero until the media has loaded.
  Duration get duration;
}
