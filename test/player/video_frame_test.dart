import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/video_frame.dart';
import 'package:jitter/widgets/focus_ring.dart';

/// A stand-in for media_kit's video: its own focus, and Space while it
/// has it.
class _FakeVideo extends StatelessWidget {
  const new({required this.played});

  final VoidCallback played;

  @override
  Widget build(BuildContext context) {
    return CallbackShortcuts(
      bindings: {const SingleActivator(LogicalKeyboardKey.space): played},
      child: const Focus(debugLabel: 'video', child: SizedBox.expand()),
    );
  }
}

/// Pumps a button, then the frame, around [child].
Future<void> pumpFrame(
  WidgetTester tester, {
  required bool childTakesFocus,
  required Widget child,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: Column(
        children: [
          TextButton(onPressed: () {}, child: const Text('Before')),
          Expanded(
            child: VideoFrame(childTakesFocus: childTakesFocus, child: child),
          ),
        ],
      ),
    ),
  ),
);

bool ringShown(WidgetTester tester) =>
    tester.widget<FocusRing>(find.byType(FocusRing)).shown;

Future<void> tab(WidgetTester tester) async {
  await tester.sendKeyEvent(LogicalKeyboardKey.tab);
  await tester.pump();
}

void main() {
  testWidgets('while loading the box is a labelled Tab stop, ringed', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    await pumpFrame(tester, childTakesFocus: false, child: const SizedBox());
    expect(ringShown(tester), isFalse);

    await tab(tester);
    await tab(tester);

    expect(primaryFocus?.debugLabel, 'Video frame');
    expect(ringShown(tester), isTrue);
    expect(
      tester.getSemantics(find.bySemanticsLabel('Video player')),
      isSemantics(label: 'Video player', isFocusable: true, isFocused: true),
    );
    semantics.dispose();
  });

  testWidgets('the video is the stop once there, its keys reach it, ringed', (
    tester,
  ) async {
    var plays = 0;
    await pumpFrame(
      tester,
      childTakesFocus: true,
      child: _FakeVideo(played: () => plays++),
    );

    await tab(tester);
    await tab(tester);

    expect(primaryFocus?.debugLabel, 'video');
    expect(ringShown(tester), isTrue);
    await tester.sendKeyEvent(LogicalKeyboardKey.space);
    expect(plays, 1);

    // Past the video, round to the button: the frame is no stop of its own.
    await tab(tester);
    expect(primaryFocus?.debugLabel, isNot(anyOf('video', 'Video frame')));
    expect(ringShown(tester), isFalse);
  });

  testWidgets('the box hands its focus to the video when it comes', (
    tester,
  ) async {
    await pumpFrame(tester, childTakesFocus: false, child: const SizedBox());
    await tab(tester);
    await tab(tester);
    expect(primaryFocus?.debugLabel, 'Video frame');

    await pumpFrame(
      tester,
      childTakesFocus: true,
      child: _FakeVideo(played: () {}),
    );
    await tester.pump();

    expect(primaryFocus?.debugLabel, 'video');
    expect(ringShown(tester), isTrue);
  });
}
