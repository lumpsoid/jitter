import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/widgets/escape_goes_back.dart';

/// Pumps a home page and pushes [page] over it, in [EscapeGoesBack].
Future<void> pumpPushed(WidgetTester tester, Widget page) async {
  await tester.pumpWidget(const MaterialApp(home: Text('Home')));
  Navigator.of(
    tester.element(find.text('Home')),
  ).push(MaterialPageRoute<void>(builder: (_) => EscapeGoesBack(child: page)));
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('Escape goes back off the page, ahead of the focused widget', (
    tester,
  ) async {
    await pumpPushed(
      tester,
      // Like media_kit's video, which takes Escape while focused.
      Focus(
        autofocus: true,
        onKeyEvent: (_, event) => event.logicalKey == LogicalKeyboardKey.escape
            ? KeyEventResult.handled
            : KeyEventResult.ignored,
        child: const Text('Page'),
      ),
    );

    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();

    expect(find.text('Page'), findsNothing);
    expect(find.text('Home'), findsOneWidget);
  });

  testWidgets("Escape on a dialog above the page is the dialog's", (
    tester,
  ) async {
    await pumpPushed(tester, const Text('Page'));
    unawaited(
      showDialog<void>(
        context: tester.element(find.text('Page')),
        builder: (_) => const AlertDialog(content: Text('Dialog')),
      ),
    );
    await tester.pumpAndSettle();

    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();

    expect(find.text('Dialog'), findsNothing);
    expect(find.text('Page'), findsOneWidget);
  });

  testWidgets('a PopScope under it has its say', (tester) async {
    var refused = 0;
    await pumpPushed(
      tester,
      PopScope(
        canPop: false,
        onPopInvokedWithResult: (didPop, _) => refused++,
        child: const Text('Page'),
      ),
    );

    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();

    expect(refused, 1);
    expect(find.text('Page'), findsOneWidget);
  });

  testWidgets('given onEscape, Escape calls it instead', (tester) async {
    var escaped = 0;
    await tester.pumpWidget(
      MaterialApp(
        home: EscapeGoesBack(
          onEscape: () => escaped++,
          child: const Text('Page'),
        ),
      ),
    );

    await tester.sendKeyEvent(LogicalKeyboardKey.escape);

    expect(escaped, 1);
  });
}
