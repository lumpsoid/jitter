import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/desktop_controls.dart';
import 'package:material_ui/material_ui.dart';

/// The centre of the 800×450 box the controls fill, and a point outside it
/// but on the 800×600 test screen.
const _centre = Offset(400, 225);
const _outside = Offset(400, 550);

/// Pumps the controls filling an 800×450 box at the top left of the app,
/// so there is space outside it, with the bar of one text.
Future<void> pumpControls(
  WidgetTester tester, {
  Map<ShortcutActivator, VoidCallback> keys =
      const <ShortcutActivator, VoidCallback>{},
  Stream<bool> Function()? bufferings,
  VoidCallback? onToggleFullscreen,
}) {
  return tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: Align(
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: 800,
            height: 450,
            child: DesktopControls(
              keys: keys,
              bottomBar: const [Text('bar')],
              bufferings: bufferings ?? Stream<bool>.empty,
              onToggleFullscreen: onToggleFullscreen ?? () {},
            ),
          ),
        ),
      ),
    ),
  );
}

/// Runs the 150 ms fade through, after a change of state, in frames of
/// 50 ms: the first applies the change and starts the fade, three carry it
/// to its end, and the last builds what the end removed. A frame of no
/// time does not remove it, so the frames have time in them.
Future<void> fadeThrough(WidgetTester tester) async {
  for (var frame = 0; frame < 5; frame++) {
    await tester.pump(const Duration(milliseconds: 50));
  }
}

/// The opacity the bar is drawn at now, read from its AnimatedOpacity's fade.
double barOpacity(WidgetTester tester) {
  final fade = find.ancestor(
    of: find.text('bar'),
    matching: find.byType(AnimatedOpacity),
  );
  return tester
      .widget<FadeTransition>(
        find.descendant(of: fade, matching: find.byType(FadeTransition)),
      )
      .opacity
      .value;
}

void main() {
  testWidgets('the bar is hidden at the start', (tester) async {
    await pumpControls(tester);

    expect(find.text('bar'), findsNothing);
  });

  testWidgets('a mouse over the video shows the bar, 3 s after its last move', (
    tester,
  ) async {
    await pumpControls(tester);
    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await mouse.addPointer(location: _outside);
    await mouse.moveTo(_centre);
    await tester.pump();

    expect(find.text('bar'), findsOneWidget);

    await tester.pump(const Duration(seconds: 3));
    await fadeThrough(tester);

    expect(find.text('bar'), findsNothing);
  });

  testWidgets('the bar fades in, not at once', (tester) async {
    await pumpControls(tester);
    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await mouse.addPointer(location: _outside);
    await mouse.moveTo(_centre);
    await tester.pump();

    expect(find.text('bar'), findsOneWidget);
    expect(
      tester
          .widget<AnimatedOpacity>(
            find.ancestor(
              of: find.text('bar'),
              matching: find.byType(AnimatedOpacity),
            ),
          )
          .opacity,
      1,
    );
    expect(barOpacity(tester), lessThan(1));

    await fadeThrough(tester);

    expect(barOpacity(tester), 1);
  });

  testWidgets('another move within 3 s starts the 3 s again', (tester) async {
    await pumpControls(tester);
    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await mouse.addPointer(location: _outside);
    await mouse.moveTo(_centre);
    await tester.pump();

    await tester.pump(const Duration(seconds: 2));
    await mouse.moveTo(const Offset(410, 225));
    await tester.pump(const Duration(seconds: 2));

    expect(find.text('bar'), findsOneWidget);

    await tester.pump(const Duration(seconds: 1));
    await fadeThrough(tester);

    expect(find.text('bar'), findsNothing);
  });

  testWidgets('the mouse leaving the video hides the bar at once', (
    tester,
  ) async {
    await pumpControls(tester);
    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await mouse.addPointer(location: _outside);
    await mouse.moveTo(_centre);
    await tester.pump();

    expect(find.text('bar'), findsOneWidget);

    await mouse.moveTo(_outside);
    await fadeThrough(tester);

    expect(find.text('bar'), findsNothing);
  });

  testWidgets('the keys work without a tap, the focus being given at once', (
    tester,
  ) async {
    var count = 0;
    await pumpControls(
      tester,
      keys: {const SingleActivator(LogicalKeyboardKey.space): () => count++},
    );

    await tester.sendKeyEvent(LogicalKeyboardKey.space);

    expect(count, 1);
  });

  testWidgets(
    'a given focus node holds the keys over a sibling that autofocuses',
    (tester) async {
      var count = 0;
      final focusNode = FocusNode();
      addTearDown(focusNode.dispose);
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Column(
              children: [
                // Takes the focus first, as the page's own shortcuts do.
                const Focus(
                  autofocus: true,
                  child: SizedBox(width: 10, height: 10),
                ),
                Expanded(
                  child: DesktopControls(
                    keys: {
                      const SingleActivator(LogicalKeyboardKey.space): () =>
                          count++,
                    },
                    bottomBar: const [Text('bar')],
                    bufferings: Stream<bool>.empty,
                    onToggleFullscreen: () {},
                    focusNode: focusNode,
                  ),
                ),
              ],
            ),
          ),
        ),
      );
      expect(focusNode.hasFocus, isFalse);

      await tester.sendKeyEvent(LogicalKeyboardKey.space);
      expect(count, 0);

      focusNode.requestFocus();
      await tester.pump();
      await tester.sendKeyEvent(LogicalKeyboardKey.space);

      expect(count, 1);
    },
  );

  testWidgets('two quick clicks on the video toggle fullscreen once', (
    tester,
  ) async {
    var toggles = 0;
    await pumpControls(tester, onToggleFullscreen: () => toggles++);

    await tester.tapAt(_centre);
    await tester.pump(const Duration(milliseconds: 50));
    await tester.tapAt(_centre);

    expect(toggles, 1);

    // A third quick click starts a new pair; it does not toggle again.
    await tester.pump(const Duration(milliseconds: 50));
    await tester.tapAt(_centre);
    await tester.pump(const Duration(milliseconds: 500));

    expect(toggles, 1);
  });

  testWidgets('one click on the video does not toggle fullscreen', (
    tester,
  ) async {
    var toggles = 0;
    await pumpControls(tester, onToggleFullscreen: () => toggles++);

    await tester.tapAt(_centre);
    await tester.pump(const Duration(milliseconds: 500));

    expect(toggles, 0);
  });

  testWidgets('buffering shows the spinner while the bar is hidden', (
    tester,
  ) async {
    final buffering = StreamController<bool>();
    addTearDown(buffering.close);
    await pumpControls(tester, bufferings: () => buffering.stream);

    buffering.add(true);
    await fadeThrough(tester);

    expect(find.text('bar'), findsNothing);
    expect(find.byType(CircularProgressIndicator), findsOneWidget);

    buffering.add(false);
    await fadeThrough(tester);

    expect(find.byType(CircularProgressIndicator), findsNothing);
  });
}
