import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/history/history_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';

import 'fixtures.dart';

void main() {
  test('has no history until the first snapshot', () {
    expect(HistoryHolder().history, isNull);
  });

  test('a snapshot replaces the history and notifies', () {
    final holder = HistoryHolder();
    var notified = 0;
    holder
      ..addListener(() => notified++)
      ..apply(
        snapshot(
          days: [
            day('Today', [entry(1)]),
          ],
        ),
      )
      ..apply(
        snapshot(
          days: [
            day('Yesterday', [entry(2)]),
          ],
        ),
      );

    expect(holder.history?.days.map((d) => d.label), ['Yesterday']);
    expect(notified, 2);
  });

  test('a failure goes out once each, history untouched', () {
    final holder = HistoryHolder()
      ..apply(
        snapshot(
          days: [
            day('Today', [entry(1)]),
          ],
        ),
      );
    final messages = <String>[];
    holder.failures.listen(messages.add);
    var notified = 0;
    holder
      ..addListener(() => notified++)
      ..apply(const HistoryUpdateFailed(message: 'nope'))
      ..apply(const HistoryUpdateFailed(message: 'nope'));

    expect(messages, ['nope', 'nope']);
    expect(notified, 0);
    expect(holder.history?.days.single.label, 'Today');
  });

  test('removals send what they remove', () {
    final visits = <int>[];
    final targets = <PlaybackTarget>[];
    var cleared = 0;
    HistoryHolder(
        sendRemoveVisit: (signal) => visits.add(signal.id),
        sendRemoveFromHistory: (signal) => targets.add(signal.target),
        sendClearHistory: (_) => cleared++,
      )
      ..removeVisit(entry(7))
      ..removeFromHistory(entry(8, vod: '42'))
      ..clear();

    expect(visits, [7]);
    expect(targets.single, isA<PlaybackTargetVod>());
    expect((targets.single as PlaybackTargetVod).id, '42');
    expect(cleared, 1);
  });
}
