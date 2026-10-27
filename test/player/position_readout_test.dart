import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/live.dart';
import 'package:jitter/player/position_readout.dart';
import 'package:jitter/src/bindings/bindings.dart';

/// Pumps the readout at an hour into a 6-hour VOD. Each typed time sent
/// is added to the list returned; [answers] stands in for Rust's.
Future<List<String>> pumpReadout(
  WidgetTester tester,
  StreamController<GoToTimeAnswer> answers,
) async {
  final sent = <String>[];
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: PositionReadout(
          positions: Live(
            now: () => const Duration(hours: 1),
            changes: Stream.empty,
          ),
          durations: Live(
            now: () => const Duration(hours: 6),
            changes: Stream.empty,
          ),
          answers: () => answers.stream,
          onGoTo: sent.add,
          fontSize: 14,
        ),
      ),
    ),
  );
  await tester.pump();
  return sent;
}

void main() {
  late StreamController<GoToTimeAnswer> answers;

  setUp(() => answers = StreamController.broadcast());
  tearDown(() => answers.close());

  testWidgets('it reads where the VOD plays and how long it is', (
    tester,
  ) async {
    await pumpReadout(tester, answers);

    expect(find.text('1:00:00 / 6:00:00'), findsOneWidget);
    expect(find.byTooltip('Go to a time'), findsOneWidget);
  });

  testWidgets('its first frame reads the time now, then follows it', (
    tester,
  ) async {
    final positions = StreamController<Duration>.broadcast();
    addTearDown(positions.close);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: PositionReadout(
            positions: Live(
              now: () => const Duration(hours: 1),
              changes: () => positions.stream,
            ),
            durations: Live(
              now: () => const Duration(hours: 6),
              changes: Stream.empty,
            ),
            answers: () => answers.stream,
            onGoTo: (_) {},
            fontSize: 14,
          ),
        ),
      ),
    );
    expect(find.text('1:00:00 / 6:00:00'), findsOneWidget);

    positions.add(const Duration(hours: 2));
    await tester.pump();
    expect(find.text('2:00:00 / 6:00:00'), findsOneWidget);
  });

  testWidgets('a tap asks for a time, sent with Enter or Go', (tester) async {
    final sent = await pumpReadout(tester, answers);

    await tester.tap(find.text('1:00:00 / 6:00:00'));
    await tester.pumpAndSettle();
    expect(find.text('Go to time'), findsOneWidget);

    await tester.enterText(find.byType(TextField), '1h23m');
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await tester.tap(find.text('Go'));

    expect(sent, ['1h23m', '1h23m']);
  });

  testWidgets('the dialog closes once the player went there', (tester) async {
    await pumpReadout(tester, answers);
    await tester.tap(find.text('1:00:00 / 6:00:00'));
    await tester.pumpAndSettle();

    answers.add(const GoToTimeAnswerWent());
    await tester.pumpAndSettle();

    expect(find.text('Go to time'), findsNothing);
  });

  testWidgets('a rejected time says why and stays to be fixed', (tester) async {
    await pumpReadout(tester, answers);
    await tester.tap(find.text('1:00:00 / 6:00:00'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), '9h');

    answers.add(
      const GoToTimeAnswerRejected(message: 'The VOD ends at 6:00:00.'),
    );
    await tester.pump();
    expect(find.text('Go to time'), findsOneWidget);
    expect(find.text('The VOD ends at 6:00:00.'), findsOneWidget);

    await tester.enterText(find.byType(TextField), '5h');
    await tester.pump();
    expect(find.text('The VOD ends at 6:00:00.'), findsNothing);
  });

  testWidgets('Cancel closes the dialog without sending', (tester) async {
    final sent = await pumpReadout(tester, answers);
    await tester.tap(find.text('1:00:00 / 6:00:00'));
    await tester.pumpAndSettle();

    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();

    expect(find.text('Go to time'), findsNothing);
    expect(sent, isEmpty);
  });
}
