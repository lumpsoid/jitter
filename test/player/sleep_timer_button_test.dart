import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/sleep_timer_button.dart';
import 'package:jitter/src/bindings/bindings.dart';

const _presets = [
  SleepPreset(minutes: 15, label: '15 minutes'),
  SleepPreset(minutes: 30, label: '30 minutes'),
];

Future<void> pumpButton(
  WidgetTester tester, {
  String? left,
  ValueChanged<int>? onArm,
  VoidCallback? onTurnOff,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      appBar: AppBar(
        actions: [
          SleepTimerButton(
            sleep: PlaybackSleep(presets: _presets, left: left),
            onArm: onArm ?? (_) {},
            onTurnOff: onTurnOff ?? () {},
          ),
        ],
      ),
    ),
  ),
);

void main() {
  testWidgets('unarmed, it offers the presets and no way to turn off', (
    tester,
  ) async {
    final armed = <int>[];
    await pumpButton(tester, onArm: armed.add);

    expect(find.byIcon(Icons.bedtime_outlined), findsOneWidget);
    expect(find.byTooltip('Sleep timer'), findsOneWidget);
    await tester.tap(find.byType(SleepTimerButton));
    await tester.pumpAndSettle();
    expect(find.text('15 minutes'), findsOneWidget);
    expect(find.text('Turn off timer'), findsNothing);

    await tester.tap(find.text('30 minutes'));
    await tester.pumpAndSettle();
    expect(armed, [30]);
  });

  testWidgets('armed, it shows the time left and turns off', (tester) async {
    var turnedOff = 0;
    await pumpButton(tester, left: '23 min left', onTurnOff: () => turnedOff++);

    expect(find.byIcon(Icons.bedtime), findsOneWidget);
    expect(find.byTooltip('Sleep timer: 23 min left'), findsOneWidget);
    await tester.tap(find.byType(SleepTimerButton));
    await tester.pumpAndSettle();
    expect(find.text('23 min left'), findsOneWidget);
    expect(find.text('15 minutes'), findsOneWidget);

    await tester.tap(find.text('Turn off timer'));
    await tester.pumpAndSettle();
    expect(turnedOff, 1);
  });
}
