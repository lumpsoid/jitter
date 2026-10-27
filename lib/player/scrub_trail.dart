import 'package:jitter/player/scrub.dart';

/// How long a finger's lift off the glass moves it before it is up: its
/// roll, not the user's aim.
const liftOff = Duration(milliseconds: 100);

/// Where a touch drag has been lately, each [Scrub] at the time of its
/// pointer event, so its end can undo the lift: a finger rolls a few
/// pixels as it comes up, which at a slowed speed still seeks seconds off
/// the spot it was held at.
class ScrubTrail {
  /// Starting at [scrub], pressed at [at].
  new(Duration at, Scrub scrub) : _spots = [(at, scrub)];

  /// Oldest first; only the newest [liftOff] and the one spot before it
  /// are kept, all [settled] can need.
  final List<(Duration, Scrub)> _spots;

  /// Moved to [scrub] at [at].
  void add(Duration at, Scrub scrub) {
    _spots.add((at, scrub));
    while (_spots.length > 1 && _spots[1].$1 <= at - liftOff) {
      _spots.removeAt(0);
    }
  }

  /// Where the drag was [liftOff] before it lifted at [upAt]: its last spot
  /// from then, or where it started when it is younger than that.
  Scrub settled(Duration upAt) {
    final before = upAt - liftOff;
    return _spots
        .lastWhere((spot) => spot.$1 <= before, orElse: () => _spots.first)
        .$2;
  }
}
