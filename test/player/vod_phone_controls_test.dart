import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/vod_phone_controls.dart';

/// Long enough for a lone tap to be no double tap.
const _tapSettles = Duration(milliseconds: 400);

/// The controls over a 600×300 video at the top-left of the screen, with
/// stand-in parts; each step a double tap goes is added to [seeks].
Future<void> pumpControls(
  WidgetTester tester, {
  List<Duration>? seeks,
  Stream<bool>? bufferings,
  Widget? mode,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      home: Material(
        child: Align(
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: 600,
            height: 300,
            child: VodPhoneControls(
              playPause: const Text('play'),
              readout: const Text('readout'),
              fullscreen: const Text('fullscreen'),
              bar: const SizedBox(
                key: Key('bar'),
                height: 36,
                child: ColoredBox(color: Color(0xFF00FF00)),
              ),
              bufferings: () => bufferings ?? Stream.value(false),
              onSeekBy: (by) => seeks?.add(by),
              mode: mode,
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pump();
}

Future<void> tapVideo(
  WidgetTester tester, [
  Offset at = const Offset(300, 80),
]) async {
  await tester.tapAt(at);
  await tester.pump(_tapSettles);
}

Future<void> doubleTap(WidgetTester tester, Offset at) async {
  await tester.tapAt(at);
  await tester.pump(const Duration(milliseconds: 50));
  await tester.tapAt(at);
  await tester.pump();
}

void main() {
  testWidgets('a tap shows them and another hides them', (tester) async {
    await pumpControls(tester);
    expect(find.text('play'), findsNothing);

    await tapVideo(tester);
    expect(find.text('play'), findsOneWidget);
    expect(find.text('readout'), findsOneWidget);

    await tapVideo(tester);
    await tester.pumpAndSettle();
    expect(find.text('play'), findsNothing);
  });

  testWidgets('they hide 3 s after the last finger lifts', (tester) async {
    await pumpControls(tester);
    await tapVideo(tester);

    await tester.pump(const Duration(seconds: 2));
    expect(find.text('play'), findsOneWidget);
    await tester.pump(const Duration(seconds: 1));
    await tester.pumpAndSettle();
    expect(find.text('play'), findsNothing);
  });

  testWidgets('a finger held on the bar keeps them, however long', (
    tester,
  ) async {
    await pumpControls(tester);
    await tapVideo(tester);

    final drag = await tester.startGesture(
      tester.getCenter(find.byKey(const Key('bar'))),
    );
    await tester.pump(const Duration(seconds: 10));
    await drag.moveBy(const Offset(40, -60));
    await tester.pump(const Duration(seconds: 10));
    expect(find.text('play'), findsOneWidget);

    await drag.up();
    await tester.pump(const Duration(seconds: 2));
    expect(find.text('play'), findsOneWidget);
    await tester.pump(const Duration(seconds: 1));
    await tester.pumpAndSettle();
    expect(find.text('play'), findsNothing);
  });

  testWidgets('a double tap on a side goes 10 s back or on and says so', (
    tester,
  ) async {
    final seeks = <Duration>[];
    await pumpControls(tester, seeks: seeks);

    await doubleTap(tester, const Offset(50, 150));
    expect(find.text('−10 s'), findsOneWidget);
    await doubleTap(tester, const Offset(550, 150));
    expect(find.text('+10 s'), findsOneWidget);
    // The middle third is not a side.
    await doubleTap(tester, const Offset(300, 150));
    // The flash goes after a while.
    await tester.pump(const Duration(seconds: 1));

    expect(seeks, const [Duration(seconds: -10), Duration(seconds: 10)]);
    expect(find.text('+10 s'), findsNothing);
    // A double tap is not a tap: the controls stayed hidden.
    expect(find.text('play'), findsNothing);
  });

  testWidgets('while it buffers a spinner shows in place of play', (
    tester,
  ) async {
    final bufferings = StreamController<bool>(sync: true);
    addTearDown(bufferings.close);
    await pumpControls(tester, bufferings: bufferings.stream);
    await tapVideo(tester);

    bufferings.add(true);
    await tester.pump();
    expect(find.byType(CircularProgressIndicator), findsOneWidget);
    expect(find.text('play'), findsNothing);

    bufferings.add(false);
    await tester.pump();
    expect(find.byType(CircularProgressIndicator), findsNothing);
    expect(find.text('play'), findsOneWidget);
  });

  testWidgets('without a mode the readout row is readout, then fullscreen', (
    tester,
  ) async {
    await pumpControls(tester);
    await tapVideo(tester);
    expect(find.text('readout'), findsOneWidget);
    expect(find.text('fullscreen'), findsOneWidget);
    expect(find.text('mode'), findsNothing);
  });

  testWidgets('a mode sits after the readout, away from fullscreen', (
    tester,
  ) async {
    await pumpControls(tester, mode: const Text('mode'));
    expect(find.text('mode'), findsNothing);

    await tapVideo(tester);
    final readout = tester.getRect(find.text('readout'));
    final mode = tester.getRect(find.text('mode'));
    final fullscreen = tester.getRect(find.text('fullscreen'));
    expect(mode.left, greaterThanOrEqualTo(readout.right));
    expect(mode.center.dy, readout.center.dy);
    // Away from fullscreen: the row's free space lies between them.
    expect(fullscreen.left - mode.right, greaterThan(200));
  });
}
