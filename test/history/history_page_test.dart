import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/history/history_holder.dart';
import 'package:jitter/history/history_page.dart';
import 'package:jitter/src/bindings/bindings.dart';

import 'fixtures.dart';

Future<void> pumpPage(
  WidgetTester tester,
  HistoryHolder holder, {
  void Function(String login)? onOpenLive,
  void Function(String id)? onOpenVod,
  void Function(String login)? onOpenChannel,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: HistoryPage(
        history: holder,
        onOpenLive: onOpenLive ?? (_) {},
        onOpenVod: onOpenVod ?? (_) {},
        onOpenChannel: onOpenChannel ?? (_) {},
      ),
    ),
  ),
);

void main() {
  testWidgets('spins until the first snapshot', (tester) async {
    await pumpPage(tester, HistoryHolder());

    expect(find.byType(CircularProgressIndicator), findsOneWidget);
  });

  testWidgets('says what shows up here when nothing was watched', (
    tester,
  ) async {
    await pumpPage(tester, HistoryHolder()..apply(snapshot()));

    expect(find.textContaining('Nothing watched yet'), findsOneWidget);
    final clear = tester.widget<IconButton>(
      find.widgetWithIcon(IconButton, Icons.delete_sweep_outlined),
    );
    expect(clear.onPressed, isNull);
  });

  testWidgets('the timeline lists every visit under its day', (tester) async {
    final holder = HistoryHolder()
      ..apply(
        snapshot(
          days: [
            day('Today', [
              entry(3, title: 'back again', when: '16:00'),
              entry(2, title: 'a vod', vod: '42', streamed: 'Oct 24'),
            ]),
            day('Yesterday', [entry(1, title: 'back again', when: '09:00')]),
          ],
          unique: [
            entry(3, title: 'back again'),
            entry(2, title: 'a vod'),
          ],
        ),
      );
    await pumpPage(tester, holder);

    expect(find.text('Today'), findsOneWidget);
    expect(find.text('Yesterday'), findsOneWidget);
    expect(find.text('back again'), findsNWidgets(2));
    expect(find.text('Live · XQC · Grand Theft Auto V'), findsNothing);
    expect(
      find.text('Live · XQC · Grand Theft Auto V\n16:00 · watched 40m'),
      findsOneWidget,
    );
    expect(find.textContaining('VOD · Oct 24 · XQC'), findsOneWidget);
    expect(
      tester.getTopLeft(find.text('Today')).dy,
      lessThan(tester.getTopLeft(find.text('Yesterday')).dy),
    );
  });

  testWidgets('the unique view lists each once, without days', (tester) async {
    final holder = HistoryHolder()
      ..apply(
        snapshot(
          days: [
            day('Today', [entry(3, title: 'back again')]),
            day('Yesterday', [entry(1, title: 'back again')]),
          ],
          unique: [entry(3, title: 'back again', when: 'Today, 14:02')],
        ),
      );
    await pumpPage(tester, holder);
    await tester.tap(find.text('Unique'));
    await tester.pump();

    expect(find.text('back again'), findsOneWidget);
    expect(find.text('Yesterday'), findsNothing);
    expect(find.textContaining('Today, 14:02'), findsOneWidget);
  });

  testWidgets('tapping an entry plays it; its avatar opens the channel', (
    tester,
  ) async {
    final live = <String>[];
    final vods = <String>[];
    final channels = <String>[];
    final holder = HistoryHolder()
      ..apply(
        snapshot(
          days: [
            day('Today', [
              entry(2, title: 'a stream'),
              entry(1, title: 'a vod', vod: '42', login: 'other'),
            ]),
          ],
        ),
      );
    await pumpPage(
      tester,
      holder,
      onOpenLive: live.add,
      onOpenVod: vods.add,
      onOpenChannel: channels.add,
    );

    await tester.tap(find.text('a stream'));
    await tester.tap(find.text('a vod'));
    await tester.tap(find.text('O'));

    expect(live, ['xqc']);
    expect(vods, ['42']);
    expect(channels, ['other']);
  });

  testWidgets('removes a visit on the timeline, everything in unique', (
    tester,
  ) async {
    final visits = <int>[];
    final targets = <Object>[];
    final holder =
        HistoryHolder(
          sendRemoveVisit: (signal) => visits.add(signal.id),
          sendRemoveFromHistory: (signal) => targets.add(signal.target),
        )..apply(
          snapshot(
            days: [
              day('Today', [entry(5)]),
            ],
            unique: [entry(5)],
          ),
        );
    await pumpPage(tester, holder);

    await tester.tap(find.byTooltip('More'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Remove this visit'));
    await tester.pumpAndSettle();
    expect(visits, [5]);

    await tester.tap(find.text('Unique'));
    await tester.pump();
    await tester.tap(find.byTooltip('More'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Remove from history'));
    await tester.pumpAndSettle();
    expect(targets, hasLength(1));
  });

  testWidgets('clearing asks first, and only Clear clears', (tester) async {
    var cleared = 0;
    final holder = HistoryHolder(sendClearHistory: (_) => cleared++)
      ..apply(
        snapshot(
          days: [
            day('Today', [entry(1)]),
          ],
        ),
      );
    await pumpPage(tester, holder);

    await tester.tap(find.byTooltip('Clear the history'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();
    expect(cleared, 0);

    await tester.tap(find.byTooltip('Clear the history'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Clear'));
    await tester.pumpAndSettle();
    expect(cleared, 1);
  });

  testWidgets('a failed removal shows as a snackbar', (tester) async {
    final holder = HistoryHolder()..apply(snapshot());
    await pumpPage(tester, holder);

    holder.apply(const HistoryUpdateFailed(message: 'Could not remove'));
    await tester.pump();

    expect(find.text('Could not remove'), findsOneWidget);
  });
}
