import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/widgets/refresh_key.dart';

Future<void> pressCtrlR(WidgetTester tester, {bool shift = false}) async {
  await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
  if (shift) await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
  await tester.sendKeyEvent(LogicalKeyboardKey.keyR);
  if (shift) await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
  await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
}

void main() {
  testWidgets('Ctrl+R refreshes the page, R or Ctrl+Shift+R does not', (
    tester,
  ) async {
    var refreshed = 0;
    await tester.pumpWidget(
      MaterialApp(
        home: RefreshKey(
          onRefresh: () => refreshed++,
          child: const Text('Page'),
        ),
      ),
    );

    await pressCtrlR(tester);
    expect(refreshed, 1);

    await tester.sendKeyEvent(LogicalKeyboardKey.keyR);
    await pressCtrlR(tester, shift: true);
    expect(refreshed, 1);
  });

  testWidgets('a page pushed above it has the key', (tester) async {
    var refreshed = 0;
    await tester.pumpWidget(
      MaterialApp(
        home: RefreshKey(
          onRefresh: () => refreshed++,
          child: const Text('Home'),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final navigator = Navigator.of(tester.element(find.text('Home')));
    final pushed = navigator.push(
      MaterialPageRoute<void>(builder: (_) => const Text('Above')),
    );
    await tester.pumpAndSettle();

    await pressCtrlR(tester);
    expect(refreshed, 0);

    navigator.pop();
    await pushed;
    await tester.pumpAndSettle();
    await pressCtrlR(tester);
    expect(refreshed, 1);
  });
}
