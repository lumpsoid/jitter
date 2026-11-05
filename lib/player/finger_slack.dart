/// How far a finger can wander along the bar without moving its drag: a
/// held finger's tremor, but not a deliberate move.
const double fingerSlack = 8;

/// A finger's spot along the seek bar with [fingerSlack] of play, as on a
/// rope: the spot stays put while the finger wanders within the slack of
/// it, and is pulled along, the slack's length behind, once it goes
/// further. A sweep moves it as fast as the finger; a tremor not at all.
class FingerSlack {
  /// Pressed at [x].
  new(double x) : _held = x;

  double _held;

  /// Moved to [x]: where the drag's spot is now.
  double follow(double x) {
    if (x > _held + fingerSlack) _held = x - fingerSlack;
    if (x < _held - fingerSlack) _held = x + fingerSlack;
    return _held;
  }
}
