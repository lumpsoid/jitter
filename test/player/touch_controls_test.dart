import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/controls_fade.dart';
import 'package:jitter/player/touch_controls.dart';
import 'package:material_ui/material_ui.dart';

/// The centre of the 800×450 box the controls fill, where the centre button
/// is, and a point on the video away from it, where a tap shows the controls.
const _centre = Offset(400, 225);
const _beside = Offset(100, 225);

const _centreKey = Key('center');

/// Pumps the controls filling an 800×450 box at the top left of the app, so
/// there is space outside it, with a centre button that calls [onPressed] and
/// the bar of one text.
Future<void> pumpControls(
  WidgetTester tester, {
  VoidCallback? onPressed,
  Stream<bool> Function()? bufferings,
}) {
  return tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: Align(
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: 800,
            height: 450,
            child: TouchControls(
              center: IconButton(
                key: _centreKey,
                icon: const Icon(Icons.play_arrow),
                onPressed: onPressed ?? () {},
              ),
              bottomBar: const [Text('bar')],
              bufferings: bufferings ?? Stream<bool>.empty,
            ),
          ),
        ),
      ),
    ),
  );
}

/// Runs the 300 ms fade through, after a change of state, in frames of 50 ms:
/// six of them carry the fade to its end, and the last two let the end remove
/// what it hid. The frames have time in them, so a frame of no time does not
/// run the fade.
Future<void> fadeThrough(WidgetTester tester) async {
  for (var frame = 0; frame < 8; frame++) {
    await tester.pump(const Duration(milliseconds: 50));
  }
}

/// How far the bar has faded in now, read from the [ControlsFade] above it:
/// 0 hidden, 1 shown.
double barFade(WidgetTester tester) {
  return tester
      .widget<ControlsFade>(
        find.ancestor(
          of: find.text('bar'),
          matching: find.byType(ControlsFade),
        ),
      )
      .notifier!
      .value;
}

/// Expects no layer in the controls that draws a subtree offscreen to fade
/// it: opacity on Impeller's GLES backend stalled the video for each fade.
void expectNoOpacityLayers(WidgetTester tester) {
  for (final layer in [AnimatedOpacity, Opacity, FadeTransition]) {
    expect(
      find.descendant(
        of: find.byType(TouchControls),
        matching: find.byType(layer),
      ),
      findsNothing,
      reason: '$layer',
    );
  }
}

void main() {
  testWidgets('the controls are hidden at the start', (tester) async {
    await pumpControls(tester);

    expect(find.text('bar'), findsNothing);
  });

  testWidgets('a tap shows the controls, 3 s after the tap', (tester) async {
    await pumpControls(tester);
    await tester.tapAt(_beside);
    await tester.pump();

    expect(find.text('bar'), findsOneWidget);

    await tester.pump(const Duration(seconds: 2));

    expect(find.text('bar'), findsOneWidget);

    await tester.pump(const Duration(seconds: 1));
    await fadeThrough(tester);

    expect(find.text('bar'), findsNothing);
  });

  testWidgets('a tap while shown hides the controls at once', (tester) async {
    await pumpControls(tester);
    await tester.tapAt(_beside);
    await tester.pump(const Duration(seconds: 1));

    await tester.tapAt(_beside);
    await fadeThrough(tester);

    expect(find.text('bar'), findsNothing);
  });

  testWidgets('the controls fade in, not at once', (tester) async {
    await pumpControls(tester);
    await tester.tapAt(_beside);
    await tester.pump();

    expect(find.text('bar'), findsOneWidget);
    expect(barFade(tester), lessThan(1));

    await fadeThrough(tester);

    expect(barFade(tester), 1);
  });

  testWidgets('the controls draw no opacity layer, with the bar shown', (
    tester,
  ) async {
    await pumpControls(tester);
    await tester.tapAt(_beside);
    await tester.pump();
    await fadeThrough(tester);

    expect(find.text('bar'), findsOneWidget);
    expectNoOpacityLayers(tester);
  });

  testWidgets('a tap on the centre button presses it and keeps the controls', (
    tester,
  ) async {
    var pressed = 0;
    await pumpControls(tester, onPressed: () => pressed++);
    await tester.tapAt(_beside);
    await tester.pump();

    await tester.tap(find.byKey(_centreKey));
    await tester.pump(const Duration(milliseconds: 100));

    expect(pressed, 1);
    expect(find.text('bar'), findsOneWidget);
  });

  testWidgets(
    'a tap where the centre button is does not press it while hidden',
    (tester) async {
      var pressed = 0;
      await pumpControls(tester, onPressed: () => pressed++);

      await tester.tapAt(_centre);
      await tester.pump();

      expect(pressed, 0);
      expect(find.text('bar'), findsOneWidget);
    },
  );

  testWidgets(
    'a tap where the centre button is does not press it while fading out',
    (tester) async {
      var pressed = 0;
      await pumpControls(tester, onPressed: () => pressed++);
      await tester.tapAt(_beside);
      await tester.pump();
      // Lets the fade in start, so the hide below fades out, not at once.
      await tester.pump(const Duration(milliseconds: 150));
      await tester.tapAt(_beside);
      await tester.pump(const Duration(milliseconds: 50));

      // Still built, so the button is there to be pressed if it takes the tap.
      expect(find.byKey(_centreKey), findsOneWidget);

      await tester.tapAt(_centre);
      await tester.pump();

      expect(pressed, 0);
    },
  );

  testWidgets('buffering shows the spinner while the controls are hidden', (
    tester,
  ) async {
    final buffering = StreamController<bool>();
    addTearDown(buffering.close);
    await pumpControls(tester, bufferings: () => buffering.stream);

    buffering.add(true);
    await fadeThrough(tester);

    expect(find.text('bar'), findsNothing);
    expect(find.byType(CircularProgressIndicator), findsOneWidget);

    await tester.tapAt(_beside);
    await tester.pump();

    expect(find.text('bar'), findsOneWidget);
    expect(find.byKey(_centreKey), findsNothing);

    buffering.add(false);
    await fadeThrough(tester);

    expect(find.byType(CircularProgressIndicator), findsNothing);
    expect(find.byKey(_centreKey), findsOneWidget);
  });

  testWidgets('the controls draw no opacity layer, with the spinner shown', (
    tester,
  ) async {
    final buffering = StreamController<bool>();
    addTearDown(buffering.close);
    await pumpControls(tester, bufferings: () => buffering.stream);

    buffering.add(true);
    await fadeThrough(tester);

    expect(find.byType(CircularProgressIndicator), findsOneWidget);
    expectNoOpacityLayers(tester);
  });
}
