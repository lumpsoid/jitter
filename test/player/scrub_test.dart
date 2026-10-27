import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/scrub.dart';

/// A 6-hour VOD on a 1000-pixel bar: 21.6 s a pixel.
const _vod = Duration(hours: 6);
const _width = 1000.0;
const _pixel = Duration(milliseconds: 21600);

Scrub _start(
  double x, {
  ScrubSpeed speed = ScrubSpeed.normal,
  Duration position = const Duration(hours: 1),
}) => Scrub.start(
  x: x,
  width: _width,
  duration: _vod,
  position: position,
  speed: speed,
);

void main() {
  test('Shift slows to a tenth and Shift and Ctrl to a hundredth', () {
    expect(ScrubSpeed.held(shift: false, control: false), ScrubSpeed.normal);
    expect(ScrubSpeed.held(shift: false, control: true), ScrubSpeed.normal);
    expect(ScrubSpeed.held(shift: true, control: false), ScrubSpeed.fine);
    expect(ScrubSpeed.held(shift: true, control: true), ScrubSpeed.finest);
  });

  test('each band away from the bar slows to a tenth, then a hundredth', () {
    expect(ScrubSpeed.away(0), ScrubSpeed.normal);
    expect(ScrubSpeed.away(scrubBand - 1), ScrubSpeed.normal);
    expect(ScrubSpeed.away(-scrubBand), ScrubSpeed.fine);
    expect(ScrubSpeed.away(scrubBand * 2), ScrubSpeed.finest);
    expect(ScrubSpeed.away(-scrubBand * 20), ScrubSpeed.finest);
  });

  test('keys and distance together go at the slower', () {
    expect(ScrubSpeed.fine.slowest(ScrubSpeed.normal), ScrubSpeed.fine);
    expect(ScrubSpeed.normal.slowest(ScrubSpeed.fine), ScrubSpeed.fine);
    expect(ScrubSpeed.finest.slowest(ScrubSpeed.fine), ScrubSpeed.finest);
  });

  test('a press at the normal speed jumps to the spot', () {
    final scrub = _start(250);

    expect(scrub.target, const Duration(hours: 1, minutes: 30));
    expect(scrub.fraction, 0.25);
  });

  test('a slowed press starts from where it plays', () {
    final scrub = _start(250, speed: ScrubSpeed.fine);

    expect(scrub.target, const Duration(hours: 1));
  });

  test('a drag at the normal speed follows the pointer', () {
    final scrub = _start(250).moveTo(260, ScrubSpeed.normal);

    expect(scrub.target, const Duration(hours: 1, minutes: 30) + _pixel * 10);
  });

  test('a slowed drag moves a tenth or a hundredth of a pixel', () {
    final fine = _start(250).moveTo(260, ScrubSpeed.fine);
    final finest = _start(250).moveTo(260, ScrubSpeed.finest);

    expect(fine.target, const Duration(hours: 1, minutes: 30) + _pixel);
    expect(finest.target, const Duration(hours: 1, minutes: 30) + _pixel ~/ 10);
  });

  test('a change of speed mid-drag goes on from where the drag is', () {
    final scrub = _start(250)
        .moveTo(260, ScrubSpeed.fine)
        .moveTo(270, ScrubSpeed.normal);

    expect(scrub.target, const Duration(hours: 1, minutes: 30) + _pixel * 11);
  });

  test('a drag stays within the VOD', () {
    expect(_start(10).moveTo(-500, ScrubSpeed.normal).target, Duration.zero);
    expect(_start(990).moveTo(5000, ScrubSpeed.normal).target, _vod);
    expect(_start(-20).target, Duration.zero);
  });

  test('a bar of no width or a VOD of no length does not move', () {
    final scrub = Scrub.start(
      x: 10,
      width: 0,
      duration: _vod,
      position: const Duration(minutes: 5),
      speed: ScrubSpeed.normal,
    );

    expect(scrub.target, const Duration(minutes: 5));
    expect(scrub.moveTo(50, ScrubSpeed.normal).target, scrub.target);
    expect(
      Scrub.start(
        x: 10,
        width: _width,
        duration: Duration.zero,
        position: Duration.zero,
        speed: ScrubSpeed.normal,
      ).fraction,
      0,
    );
  });
}
