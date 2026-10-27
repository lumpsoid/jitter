import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/sleep_stopped.dart';

Widget _onBlack(Widget child) => MaterialApp(
  home: Scaffold(
    body: ColoredBox(color: Colors.black, child: child),
  ),
);

void main() {
  testWidgets('it says why in white, as a live region', (tester) async {
    final semantics = tester.ensureSemantics();
    await tester.pumpWidget(_onBlack(SleepStopped(onResume: () {})));

    final message = tester.widget<RichText>(
      find.descendant(
        of: find.text('Stopped by the sleep timer'),
        matching: find.byType(RichText),
      ),
    );
    expect(message.text.style?.color, Colors.white);
    expect(
      tester.getSemantics(find.byType(SleepStopped)),
      isSemantics(label: 'Stopped by the sleep timer', isLiveRegion: true),
    );
    semantics.dispose();
  });

  testWidgets('Resume takes the focus from the video it replaced', (
    tester,
  ) async {
    final video = FocusNode(debugLabel: 'video');
    addTearDown(video.dispose);
    await tester.pumpWidget(
      _onBlack(Focus(focusNode: video, child: const Text('video'))),
    );
    video.requestFocus();
    await tester.pump();
    expect(video.hasPrimaryFocus, isTrue);

    var resumed = 0;
    await tester.pumpWidget(_onBlack(SleepStopped(onResume: () => resumed++)));
    await tester.pump();

    final resume = find.widgetWithText(FilledButton, 'Resume');
    expect(
      Focus.of(tester.element(find.text('Resume'))).hasPrimaryFocus,
      isTrue,
    );
    await tester.tap(resume);
    expect(resumed, 1);
  });
}
