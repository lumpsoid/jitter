import 'package:jitter/player/live.dart';

/// The video player a `PlayerCommand` drives: the border a test replaces
/// with a fake. `MediaKitEngine` is the real one.
abstract interface class PlayerEngine {
  /// Plays [url] from [start], at the user's volume.
  Future<void> open(String url, {required Duration start});

  /// Stops; a fade ends with it.
  Future<void> stop();

  /// Stops what plays, if anything ever played, so the exit does not wait on
  /// the player tearing down a live stream and its video output.
  Future<void> stopForExit();

  /// Plays, if paused.
  Future<void> play();

  /// Pauses, if playing.
  Future<void> pause();

  /// Pauses if playing, else plays.
  Future<void> togglePlaying();

  /// Whether it is playing now, not paused.
  bool get isPlaying;

  /// Whether it is playing now, then each change; an `open` passes
  /// through `false`.
  Stream<bool> get playing;

  /// Goes to [to] in the VOD playing.
  Future<void> seek(Duration to);

  /// Moves the position by [by], within the media: not before its start,
  /// nor past its end, which would end it.
  Future<void> seekBy(Duration by);

  /// Steps one frame [forward] or back, while paused; playing, it does
  /// nothing.
  Future<void> stepFrame({required bool forward});

  /// Where it plays now.
  Duration get position;

  /// Where it plays.
  Live<Duration> get positions;

  /// Zero until the media has loaded.
  Duration get duration;

  /// How long the media is: zero until it has loaded.
  Live<Duration> get durations;

  /// Whether it waits for the media to load now, then each change.
  Stream<bool> get bufferings;

  /// How far it has buffered.
  Live<Duration> get buffers;

  /// Plays at [level] (0–1) times the user's own volume, which stays
  /// theirs; a [level] of 1 ends the fade.
  Future<void> setFade(double level);

  /// Mutes, or unmutes back to the volume it was muted at; mid-fade, under
  /// the fade.
  Future<void> toggleMute();

  /// Moves the volume by [by] (of 0–100); mid-fade, under the fade.
  Future<void> nudgeVolume(double by);

  /// The user's volume (0–100) now, then each change; under a fade, the one
  /// it fades from.
  Live<double> get volumes;

  /// Sets the user's volume to [to] (0–100); mid-fade, under the fade.
  Future<void> setVolume(double to);
}
