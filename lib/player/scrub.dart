import 'package:flutter/foundation.dart';

/// How far from the bar each slower band starts, a finger's width apart.
const double scrubBand = 50;

/// How fast a drag along the seek bar moves the position, by the keys held
/// or by how far from the bar the pointer has gone: across a long VOD each
/// pixel of the bar is many seconds, so a precise spot needs a slower drag.
enum ScrubSpeed {
  /// The position follows the pointer across the bar.
  normal(1, ''),

  /// A band away from the bar: half the bar's pace.
  half(0.5, '×1/2'),

  /// Two bands away: a quarter.
  quarter(0.25, '×1/4'),

  /// Shift, or three bands away: a tenth.
  fine(0.1, '×1/10'),

  /// Shift and Ctrl: a hundredth.
  finest(0.01, '×1/100');

  new(this.gain, this.label);

  factory held({required bool shift, required bool control}) => !shift
      ? normal
      : control
      ? finest
      : fine;

  /// [distance] from the bar, up or down: a slower speed each
  /// [scrubBand] further, down to a tenth: a finger's way to slow down.
  factory away(double distance) => switch (distance.abs() ~/ scrubBand) {
    0 => normal,
    1 => half,
    2 => quarter,
    _ => fine,
  };

  /// The share of the bar's pace a drag moves at.
  final double gain;

  /// Shown by the drag while it is slowed; empty at the normal speed.
  final String label;

  /// The slower of this and [other]: keys held and the pointer's distance
  /// each slow a drag, the slower winning.
  ScrubSpeed slowest(ScrubSpeed other) => other.gain < gain ? other : this;
}

/// A drag along a seek bar over a VOD: the
/// position it would seek to, [target]. Each move goes at the speed held
/// then, from where the position is: a change of speed mid-drag never
/// jumps it.
@immutable
class Scrub {
  const new _(this.target, this._x, this._width, this._duration);

  /// Pressed at [x] on the bar while the VOD is at [position]: at the
  /// normal speed the position jumps to [x]; slowed, it starts where it
  /// is, for fine moves from there.
  factory start({
    required double x,
    required double width,
    required Duration duration,
    required Duration position,
    required ScrubSpeed speed,
  }) {
    final target = speed == ScrubSpeed.normal && width > 0
        ? duration * (x / width).clamp(0.0, 1.0)
        : position;
    return Scrub._(_within(target, duration), x, width, duration);
  }

  final Duration target;
  final double _x;
  final double _width;
  final Duration _duration;

  /// The share of the VOD [target] is at, `0`–`1`.
  double get fraction => _duration > Duration.zero
      ? target.inMicroseconds / _duration.inMicroseconds
      : 0;

  /// Moved to [x] at [speed]: by the pixels moved, each worth the bar's
  /// share of the VOD times the speed's gain.
  Scrub moveTo(double x, ScrubSpeed speed) {
    if (_width <= 0) return this;
    final moved = _duration * ((x - _x) / _width * speed.gain);
    return Scrub._(_within(target + moved, _duration), x, _width, _duration);
  }

  static Duration _within(Duration at, Duration duration) => at < Duration.zero
      ? Duration.zero
      : at > duration
      ? duration
      : at;
}
