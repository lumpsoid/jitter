import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/logs/logs_holder.dart';
import 'package:jitter/logs/logs_page.dart';
import 'package:jitter/src/bindings/bindings.dart';

const _sessions = LogsUpdateSessions(
  dir: '/data/jitter/logs',
  sessions: [
    LogSession(
      name: '2026-11-03_14-22-05.log',
      label: '2026-11-03 14:22:05',
      current: true,
    ),
    LogSession(
      name: '2026-11-02_09-00-00.log',
      label: '2026-11-02 09:00:00',
      current: false,
    ),
  ],
);

void main() {
  late List<Object> sent;
  late LogsHolder holder;

  setUp(() {
    sent = [];
    holder = LogsHolder(sendLogsReady: sent.add, sendOpenLog: sent.add);
  });

  Future<void> pumpPage(WidgetTester tester) =>
      tester.pumpWidget(MaterialApp(home: LogsPage(logs: holder)));

  testWidgets('lists the sessions newest first, marking this run', (
    tester,
  ) async {
    await pumpPage(tester);
    expect(find.byType(CircularProgressIndicator), findsOneWidget);

    holder.apply(_sessions);
    await tester.pump();

    expect(find.text('2026-11-03 14:22:05'), findsOneWidget);
    expect(find.text('This run'), findsOneWidget);
    expect(
      tester.getTopLeft(find.text('2026-11-03 14:22:05')).dy,
      lessThan(tester.getTopLeft(find.text('2026-11-02 09:00:00')).dy),
    );
  });

  testWidgets("a session shows its log's lines once it arrives", (
    tester,
  ) async {
    holder.apply(_sessions);
    await pumpPage(tester);

    await tester.tap(find.text('2026-11-02 09:00:00'));
    await tester.pump();
    await tester.pump(const Duration(seconds: 1));
    expect((sent.single as OpenLog).name, '2026-11-02_09-00-00.log');
    expect(find.byType(CircularProgressIndicator), findsOneWidget);

    holder.apply(
      const LogsUpdateOpened(
        name: '2026-11-02_09-00-00.log',
        path: '/data/jitter/logs/2026-11-02_09-00-00.log',
        text:
            'INFO hub: play live xqc\nINFO hub: buffering ended after 2.3 s\n',
      ),
    );
    await tester.pump();

    expect(find.text('INFO hub: play live xqc'), findsOneWidget);
    expect(find.byTooltip('Copy all'), findsOneWidget);
  });

  testWidgets("another session's late log is not shown", (tester) async {
    holder.apply(_sessions);
    await pumpPage(tester);
    await tester.tap(find.text('2026-11-02 09:00:00'));
    await tester.pump();
    await tester.pump(const Duration(seconds: 1));

    holder.apply(
      const LogsUpdateOpened(
        name: '2026-11-03_14-22-05.log',
        path: '/x',
        text: 'other run',
      ),
    );
    await tester.pump();

    expect(find.text('other run'), findsNothing);
    expect(find.byType(CircularProgressIndicator), findsOneWidget);
  });
}
