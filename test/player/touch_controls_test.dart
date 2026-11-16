import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
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

/// The opacity the bar is drawn at now. The nearest FadeTransition above the
/// bar is the controls' fade; the page route's transition is further out.
double barOpacity(WidgetTester tester) {
  final fades = tester.widgetList<FadeTransition>(
    find.ancestor(of: find.text('bar'), matching: find.byType(FadeTransition)),
  );
  return fades.first.opacity.value;
}

/// The opacity the centre button is drawn at now. Its AnimatedOpacity is the
/// nearest one above it; the controls' is further out.
double centreOpacity(WidgetTester tester) {
  final fades = tester.widgetList<AnimatedOpacity>(
    find.ancestor(
      of: find.byKey(_centreKey),
      matching: find.byType(AnimatedOpacity),
    ),
  );
  return fades.first.opacity;
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
    expect(barOpacity(tester), lessThan(1));

    await fadeThrough(tester);

    expect(barOpacity(tester), 1);
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
    expect(centreOpacity(tester), 0);

    buffering.add(false);
    await fadeThrough(tester);

    expect(find.byType(CircularProgressIndicator), findsNothing);
    expect(centreOpacity(tester), 1);
  });
}
