import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/shell/home_panes.dart';

import '../widgets/refresh_key_test.dart' show pressCtrlR;

const _titles = ['Top', 'Categories', 'Favorites'];

/// Three panes, each with two buttons stacked, whose refreshes count into
/// [refreshed].
Future<void> _pumpPanes(
  WidgetTester tester,
  List<int> refreshed, {
  bool active = true,
}) async {
  tester.view.physicalSize = const Size(2400, 800);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: HomePanes(
          active: active,
          panes: [
            for (var i = 0; i < _titles.length; i++)
              (
                title: _titles[i],
                page: Column(
                  children: [
                    TextButton(onPressed: () {}, child: Text('$i a')),
                    TextButton(onPressed: () {}, child: Text('$i b')),
                  ],
                ),
                refresh: () => refreshed[i]++,
              ),
          ],
        ),
      ),
    ),
  );
}

void main() {
  testWidgets('shows each pane under its title, side by side', (tester) async {
    await _pumpPanes(tester, [0, 0, 0]);

    final lefts = [
      for (final title in _titles) tester.getTopLeft(find.text(title)).dx,
    ];
    expect(lefts, orderedEquals([...lefts]..sort()));
    expect(lefts.toSet(), hasLength(3));
  });

  testWidgets('Ctrl+R refreshes every pane while none has the focus', (
    tester,
  ) async {
    final refreshed = [0, 0, 0];
    await _pumpPanes(tester, refreshed);

    await pressCtrlR(tester);
    expect(refreshed, [1, 1, 1]);
  });

  testWidgets('Ctrl+R refreshes only the pane with the focus', (tester) async {
    final refreshed = [0, 0, 0];
    await _pumpPanes(tester, refreshed);
    Focus.of(tester.element(find.text('1 b'))).requestFocus();
    await tester.pump();

    await pressCtrlR(tester);
    expect(refreshed, [0, 1, 0]);
  });

  testWidgets('Ctrl+R is not theirs while inactive', (tester) async {
    final refreshed = [0, 0, 0];
    await _pumpPanes(tester, refreshed, active: false);

    await pressCtrlR(tester);
    expect(refreshed, [0, 0, 0]);
  });

  testWidgets('Tab goes through one pane before the next', (tester) async {
    await _pumpPanes(tester, [0, 0, 0]);

    for (final label in ['0 a', '0 b', '1 a', '1 b', '2 a', '2 b']) {
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await tester.pump();
      expect(
        Focus.of(tester.element(find.text(label))).hasPrimaryFocus,
        isTrue,
        reason: label,
      );
    }
  });
}
