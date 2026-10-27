/// The video player a `PlayerCommand` drives: the border a test replaces
/// with a fake. `MediaKitEngine` is the real one.
abstract interface class PlayerEngine {
  /// Plays [url] from [start], at the user's volume.
  Future<void> open(String url, {required Duration start});

  /// Stops; a fade ends with it.
  Future<void> stop();

  /// Plays at [level] (0–1) times the user's own volume, which stays
  /// theirs; a [level] of 1 ends the fade.
  Future<void> setFade(double level);

  /// Goes to [to] in the VOD playing.
  Future<void> seek(Duration to);

  Duration get position;

  /// Zero until the media has loaded.
  Duration get duration;
}
