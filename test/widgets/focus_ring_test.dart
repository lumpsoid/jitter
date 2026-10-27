import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/widgets/focus_ring.dart';

DecoratedBox _box(WidgetTester tester) => tester.widget<DecoratedBox>(
  find.descendant(
    of: find.byType(FocusRing),
    matching: find.byType(DecoratedBox),
  ),
);

Border? _border(WidgetTester tester) =>
    (_box(tester).decoration as BoxDecoration).border as Border?;

Future<void> _pump(WidgetTester tester, {required bool shown}) =>
    tester.pumpWidget(
      MaterialApp(
        home: FocusRing(shown: shown, child: const SizedBox()),
      ),
    );

void main() {
  testWidgets('draws a ring in the primary colour, in front, while shown', (
    tester,
  ) async {
    await _pump(tester, shown: false);
    expect(_border(tester), isNull);

    await _pump(tester, shown: true);
    final primary = Theme.of(tester.element(find.byType(FocusRing)))
        .colorScheme
        .primary;
    expect(_border(tester)?.top.color, primary);
    expect(_box(tester).position, DecorationPosition.foreground);
  });
}
