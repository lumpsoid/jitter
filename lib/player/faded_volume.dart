/// The player's volume under the sleep timer's fade: the player plays at a
/// fade level times the user's own volume, and the user's volume and mute
/// stay theirs, to come back when the fade ends. Each method returns the
/// volume (0–100) to set the player to.
class FadedVolume {
  /// The user's own volume while a fade plays below it; null while none
  /// does, when the player's volume is the user's.
  double? _user;
  double _level = 1;

  /// What unmuting goes back to.
  double _unmuted = 100;

  /// Plays at [level] (0–1) of the user's volume, the player now at
  /// [current]; a [level] of 1 ends the fade. Null: nothing to set.
  double? fade(double level, {required double current}) {
    if (level >= 1) return end();
    final user = _user ??= current;
    _level = level;
    return user * level;
  }

  /// Ends the fade: the user's volume again. Null when none was playing.
  double? end() {
    final user = _user;
    _user = null;
    _level = 1;
    return user;
  }

  /// The user's volume moved by [by], kept within 0–100, the player now
  /// at [current]. Mid-fade it moves the user's volume under the fade.
  double nudge(double by, {required double current}) {
    final user = ((_user ?? current) + by).clamp(0.0, 100.0);
    if (_user != null) _user = user;
    return user * _level;
  }

  /// Mutes, or unmutes back to the volume it was muted at, the player now
  /// at [current]. Mid-fade it mutes the user's volume under the fade.
  double toggleMute({required double current}) {
    final user = _user ?? current;
    final toggled = user == 0 ? _unmuted : 0.0;
    if (user != 0) _unmuted = user;
    if (_user != null) _user = toggled;
    return toggled * _level;
  }
}
