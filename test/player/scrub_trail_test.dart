import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/scrub.dart';
import 'package:jitter/player/scrub_trail.dart';

const _vod = Duration(hours: 6);

Scrub _at(double x) => Scrub.start(
  x: x,
  width: 1000,
  duration: _vod,
  position: Duration.zero,
  speed: ScrubSpeed.normal,
);

Duration _ms(int ms) => Duration(milliseconds: ms);

void main() {
  test('it ends where the drag was before the lift began', () {
    final trail = ScrubTrail(_ms(0), _at(100))
      ..add(_ms(300), _at(200))
      ..add(_ms(560), _at(205))
      ..add(_ms(590), _at(210));

    expect(trail.settled(_ms(620)).target, _at(200).target);
  });

  test('a drag held still before it lifts ends where it is', () {
    final trail = ScrubTrail(_ms(0), _at(100))..add(_ms(300), _at(200));

    expect(trail.settled(_ms(900)).target, _at(200).target);
  });

  test('a drag younger than the lift ends where it started', () {
    final trail = ScrubTrail(_ms(0), _at(100))..add(_ms(40), _at(120));

    expect(trail.settled(_ms(80)).target, _at(100).target);
  });

  test('a long drag keeps only what its end can need', () {
    final trail = ScrubTrail(_ms(0), _at(0));
    for (var i = 1; i <= 1000; i++) {
      trail.add(_ms(i * 10), _at(i.toDouble()));
    }

    expect(trail.settled(_ms(10000)).target, _at(990).target);
  });
}
