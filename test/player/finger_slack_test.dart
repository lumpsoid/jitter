import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/finger_slack.dart';

void main() {
  test('a tremor within the slack does not move the spot', () {
    final slack = FingerSlack(100);

    for (final x in <double>[103, 96, 108, 92]) {
      expect(slack.follow(x), 100);
    }
  });

  test('a move past the slack pulls the spot along, the slack behind', () {
    final slack = FingerSlack(100);

    expect(slack.follow(150), 150 - fingerSlack);
    expect(slack.follow(300), 300 - fingerSlack);
  });

  test('turning back takes up the slack before the spot moves', () {
    final slack = FingerSlack(100)..follow(200);

    expect(slack.follow(185), 200 - fingerSlack);
    expect(slack.follow(150), 150 + fingerSlack);
  });
}
