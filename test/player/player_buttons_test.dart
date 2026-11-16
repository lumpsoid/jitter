import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/live.dart';
import 'package:jitter/player/player_buttons.dart';
import 'package:material_ui/material_ui.dart';

/// Pumps [child] centred in an app, as a page would show it.
Future<void> pumpButton(WidgetTester tester, Widget child) {
  return tester.pumpWidget(
    MaterialApp(
      home: Scaffold(body: Center(child: child)),
    ),
  );
}

double progressOf(WidgetTester tester) {
  return tester.widget<AnimatedIcon>(find.byType(AnimatedIcon)).progress.value;
}

void main() {
  testWidgets('play/pause starts where it plays, with no animation', (
    tester,
  ) async {
    final playings = StreamController<bool>();
    addTearDown(playings.close);
    await pumpButton(
      tester,
      PlayPauseButton(
        playing: Live(now: () => true, changes: () => playings.stream),
        onPressed: () {},
      ),
    );
    await tester.pump();

    expect(progressOf(tester), 1);
    expect(tester.hasRunningAnimations, isFalse);
  });

  testWidgets('play/pause paused starts at its pause end, with no animation', (
    tester,
  ) async {
    final playings = StreamController<bool>();
    addTearDown(playings.close);
    await pumpButton(
      tester,
      PlayPauseButton(
        playing: Live(now: () => false, changes: () => playings.stream),
        onPressed: () {},
      ),
    );
    await tester.pump();

    expect(progressOf(tester), 0);
    expect(tester.hasRunningAnimations, isFalse);
  });

  testWidgets('a pause animates play/pause to its end within 200 ms', (
    tester,
  ) async {
    final playings = StreamController<bool>();
    addTearDown(playings.close);
    await pumpButton(
      tester,
      PlayPauseButton(
        playing: Live(now: () => true, changes: () => playings.stream),
        onPressed: () {},
      ),
    );

    playings.add(false);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));
    // Midway: it animates, not snaps.
    expect(progressOf(tester), inExclusiveRange(0, 1));

    await tester.pump(const Duration(milliseconds: 100));
    expect(progressOf(tester), 0);
  });

  testWidgets('play/pause mounted after a hide starts where the player plays', (
    tester,
  ) async {
    // The page shows the button only while controls show, so each show mounts
    // a new one; the player has started since the page built.
    final shown = ValueNotifier<bool>(false);
    addTearDown(shown.dispose);
    var playing = false;
    await pumpButton(
      tester,
      ValueListenableBuilder<bool>(
        valueListenable: shown,
        builder: (context, show, _) => show
            ? PlayPauseButton(
                playing: Live(now: () => playing, changes: Stream<bool>.empty),
                onPressed: () {},
              )
            : const SizedBox(),
      ),
    );
    playing = true;

    shown.value = true;
    await tester.pump();

    expect(progressOf(tester), 1);
    await tester.pump(const Duration(milliseconds: 300));
    expect(progressOf(tester), 1);
  });

  testWidgets('tapping play/pause calls onPressed once', (tester) async {
    var presses = 0;
    await pumpButton(
      tester,
      PlayPauseButton(
        playing: Live(now: () => false, changes: Stream<bool>.empty),
        onPressed: () => presses++,
      ),
    );

    await tester.tap(find.byType(IconButton));

    expect(presses, 1);
  });

  testWidgets('fullscreen shows the icon for its state, and a tap presses it', (
    tester,
  ) async {
    var presses = 0;
    await pumpButton(
      tester,
      FullscreenButton(fullscreen: false, onPressed: () => presses++),
    );

    expect(find.byIcon(Icons.fullscreen), findsOneWidget);
    expect(find.byIcon(Icons.fullscreen_exit), findsNothing);

    await tester.tap(find.byType(IconButton));
    expect(presses, 1);

    await pumpButton(
      tester,
      FullscreenButton(fullscreen: true, onPressed: () => presses++),
    );

    expect(find.byIcon(Icons.fullscreen_exit), findsOneWidget);
    expect(find.byIcon(Icons.fullscreen), findsNothing);
  });

  testWidgets('the volume icon shows the level it starts at', (tester) async {
    for (final (now, icon) in [
      (0.0, Icons.volume_off),
      (30.0, Icons.volume_down),
      (80.0, Icons.volume_up),
    ]) {
      await tester.pumpWidget(const SizedBox());
      await pumpButton(
        tester,
        VolumeButton(
          volumes: Live(now: () => now, changes: Stream<double>.empty),
          onToggleMute: () {},
          onSetVolume: (_) {},
          onNudge: (_) {},
        ),
      );

      expect(find.byIcon(icon), findsOneWidget, reason: 'at $now');
    }
  });

  testWidgets('the volume icon follows a change, after its 150 ms switch', (
    tester,
  ) async {
    final volumes = StreamController<double>();
    addTearDown(volumes.close);
    await pumpButton(
      tester,
      VolumeButton(
        volumes: Live(now: () => 80, changes: () => volumes.stream),
        onToggleMute: () {},
        onSetVolume: (_) {},
        onNudge: (_) {},
      ),
    );

    volumes.add(0);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 150));
    // The switcher drops the outgoing icon on the frame after its 150 ms.
    await tester.pumpAndSettle();

    expect(find.byIcon(Icons.volume_off), findsOneWidget);
    expect(find.byIcon(Icons.volume_up), findsNothing);
  });

  testWidgets('tapping the volume icon toggles mute, once', (tester) async {
    var toggles = 0;
    await pumpButton(
      tester,
      VolumeButton(
        volumes: Live(now: () => 80, changes: Stream<double>.empty),
        onToggleMute: () => toggles++,
        onSetVolume: (_) {},
        onNudge: (_) {},
      ),
    );

    await tester.tap(find.byIcon(Icons.volume_up));

    expect(toggles, 1);
  });

  testWidgets('the slider shows while a mouse hovers, and a drag sets volume', (
    tester,
  ) async {
    final set = <double>[];
    await pumpButton(
      tester,
      VolumeButton(
        volumes: Live(now: () => 50, changes: Stream<double>.empty),
        onToggleMute: () {},
        onSetVolume: set.add,
        onNudge: (_) {},
      ),
    );
    double shown() {
      return tester
          .widget<FadeTransition>(
            find.descendant(
              of: find.byType(AnimatedOpacity),
              matching: find.byType(FadeTransition),
            ),
          )
          .opacity
          .value;
    }

    expect(shown(), 0);

    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    addTearDown(mouse.removePointer);
    await mouse.addPointer(location: Offset.zero);
    await mouse.moveTo(tester.getCenter(find.byType(IconButton)));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 150));

    expect(shown(), 1);

    await tester.drag(find.byType(Slider), const Offset(20, 0));

    expect(set, isNotEmpty);
    expect(set, everyElement(inInclusiveRange(0, 100)));
  });

  testWidgets('the wheel over the button nudges the volume up 5, down 5', (
    tester,
  ) async {
    final nudges = <double>[];
    await pumpButton(
      tester,
      VolumeButton(
        volumes: Live(now: () => 50, changes: Stream<double>.empty),
        onToggleMute: () {},
        onSetVolume: (_) {},
        onNudge: nudges.add,
      ),
    );
    final pointer = TestPointer(1, PointerDeviceKind.mouse);
    final at = tester.getCenter(find.byType(IconButton));

    await tester.sendEventToBinding(pointer.hover(at));
    await tester.sendEventToBinding(pointer.scroll(const Offset(0, -20)));
    await tester.sendEventToBinding(pointer.scroll(const Offset(0, 20)));

    expect(nudges, [5, -5]);
  });
}
