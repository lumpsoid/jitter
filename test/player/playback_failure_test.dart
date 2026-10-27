import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/playback_failure.dart';

Future<void> pumpFailure(WidgetTester tester, {VoidCallback? onRetry}) =>
    tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ColoredBox(
            color: Colors.black,
            child: PlaybackFailure(
              message: 'The stream is offline.',
              onRetry: onRetry ?? () {},
            ),
          ),
        ),
      ),
    );

void main() {
  testWidgets('a failure shows its message in white with Retry', (
    tester,
  ) async {
    await pumpFailure(tester);

    final message = tester.widget<RichText>(
      find.descendant(
        of: find.text('The stream is offline.'),
        matching: find.byType(RichText),
      ),
    );
    expect(find.text('The stream is offline.'), findsOneWidget);
    expect(message.text.style?.color, Colors.white);
    expect(find.text('Retry'), findsOneWidget);
  });

  testWidgets('a failure is a live region a screen reader announces', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    await pumpFailure(tester);

    expect(
      tester.getSemantics(find.byType(PlaybackFailure)),
      isSemantics(label: 'The stream is offline.', isLiveRegion: true),
    );
    semantics.dispose();
  });

  testWidgets('Retry tries playback again', (tester) async {
    var retries = 0;
    await pumpFailure(tester, onRetry: () => retries++);

    await tester.tap(find.text('Retry'));

    expect(retries, 1);
  });
}
