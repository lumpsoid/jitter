import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/live.dart';
import 'package:jitter/player/scrub.dart';
import 'package:jitter/player/seek_bar.dart';

const _vod = Duration(hours: 6);

/// How long a drag rests before the bar seeks there.
const _rest = Duration(milliseconds: 150);

/// Pumps the bar 1000 wide at the top-left of the screen, over a 6-hour
/// VOD playing at an hour in; each seek is added to the list returned, and
/// each [ScrubNotification]'s reading to [scrubs].
Future<List<Duration>> pumpBar(
  WidgetTester tester, {
  List<ScrubReading?>? scrubs,
}) async {
  final seeks = <Duration>[];
  tester.view
    ..physicalSize = const Size(1200, 600)
    ..devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    NotificationListener<ScrubNotification>(
      onNotification: (notification) {
        scrubs?.add(notification.reading);
        return true;
      },
      child: Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: Padding(
            padding: const EdgeInsets.only(top: 100),
            child: SizedBox(
              width: 1000,
              child: SeekBar(
                positions: Live(
                  now: () => const Duration(hours: 1),
                  changes: Stream.empty,
                ),
                durations: Live(now: () => _vod, changes: Stream.empty),
                buffers: Live(now: () => Duration.zero, changes: Stream.empty),
                onSeek: seeks.add,
              ),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pump();
  return seeks;
}

/// The bar's track at [x].
Offset _at(double x) => Offset(x, 100 + seekBarHeight / 2);

void main() {
  testWidgets('a click seeks to the spot', (tester) async {
    final seeks = await pumpBar(tester);

    await tester.tapAt(_at(250));

    expect(seeks.first, const Duration(hours: 1, minutes: 30));
  });

  testWidgets('a drag follows the pointer and shows its time', (tester) async {
    final seeks = await pumpBar(tester);

    final drag = await tester.startGesture(
      _at(250),
      kind: PointerDeviceKind.mouse,
    );
    await drag.moveTo(_at(500));
    await tester.pump(_rest);

    expect(seeks.last, const Duration(hours: 3));
    expect(find.text('3:00:00'), findsOneWidget);
    await drag.up();
  });

  testWidgets('held Shift slows a drag to a tenth, from where it plays', (
    tester,
  ) async {
    final seeks = await pumpBar(tester);

    await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
    final drag = await tester.startGesture(
      _at(250),
      kind: PointerDeviceKind.mouse,
    );
    await drag.moveTo(_at(350));
    await tester.pump(_rest);

    // 100 pixels of 21.6 s, at a tenth.
    expect(seeks.last, const Duration(hours: 1, minutes: 3, seconds: 36));
    expect(find.text('1:03:36  ×1/10'), findsOneWidget);
    await drag.up();
    await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
  });

  testWidgets('held Shift and Ctrl slow a drag to a hundredth', (tester) async {
    final seeks = await pumpBar(tester);

    await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
    await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
    final drag = await tester.startGesture(
      _at(250),
      kind: PointerDeviceKind.mouse,
    );
    await drag.moveTo(_at(350));
    await tester.pump(_rest);

    expect(
      seeks.last,
      const Duration(hours: 1, seconds: 21, milliseconds: 600),
    );
    expect(find.text('1:00:21  ×1/100'), findsOneWidget);
    await drag.up();
    await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
  });

  testWidgets('a finger taken up from the bar slows the drag by bands', (
    tester,
  ) async {
    final seeks = await pumpBar(tester);

    final drag = await tester.startGesture(_at(250));
    // A band up, straight: the position stays where the press put it.
    await drag.moveTo(_at(250) - const Offset(0, 80));
    await tester.pump(_rest);
    expect(find.text('1:30:00  ×1/10'), findsOneWidget);

    // 100 pixels of 21.6 s, at a tenth.
    await drag.moveTo(_at(350) - const Offset(0, 80));
    await tester.pump(_rest);
    expect(seeks.last, const Duration(hours: 1, minutes: 33, seconds: 36));

    // Two bands up, at a hundredth.
    await drag.moveTo(_at(450) - const Offset(0, 150));
    await tester.pump(_rest);
    expect(
      seeks.last,
      const Duration(hours: 1, minutes: 33, seconds: 57, milliseconds: 600),
    );
    expect(find.text('1:33:57  ×1/100'), findsOneWidget);

    // Back down on the bar, at its own pace again.
    await drag.moveTo(_at(550));
    await tester.pump(_rest);
    expect(
      seeks.last,
      const Duration(hours: 2, minutes: 9, seconds: 57, milliseconds: 600),
    );
    await drag.up();
  });

  /// A slowed drag held at 350, then rolled 20 pixels on as it lifts.
  Future<List<Duration>> heldThenLifted(
    WidgetTester tester,
    PointerDeviceKind kind,
  ) async {
    final seeks = await pumpBar(tester);
    final drag = await tester.createGesture(kind: kind);
    await drag.down(_at(250));
    await drag.moveTo(
      _at(250) - const Offset(0, 80),
      timeStamp: const Duration(milliseconds: 100),
    );
    await drag.moveTo(
      _at(350) - const Offset(0, 80),
      timeStamp: const Duration(milliseconds: 500),
    );
    await tester.pump(_rest);
    await drag.moveTo(
      _at(370) - const Offset(0, 80),
      timeStamp: const Duration(milliseconds: 960),
    );
    await drag.up(timeStamp: const Duration(milliseconds: 1000));
    await tester.pump(_rest);
    return seeks;
  }

  testWidgets("a finger's lift does not move where its drag ends", (
    tester,
  ) async {
    final seeks = await heldThenLifted(tester, PointerDeviceKind.touch);

    // Held at 1:33:36; the roll's 1:34:19 is undone.
    expect(seeks.last, const Duration(hours: 1, minutes: 33, seconds: 36));
    expect(find.text('1:34:19  ×1/10'), findsNothing);
  });

  testWidgets("a mouse's drag ends where it is let go", (tester) async {
    final seeks = await heldThenLifted(tester, PointerDeviceKind.mouse);

    expect(
      seeks.last,
      const Duration(hours: 1, minutes: 34, seconds: 19, milliseconds: 200),
    );
  });

  testWidgets('a sweep seeks only where it rests and where it ends', (
    tester,
  ) async {
    final seeks = await pumpBar(tester);

    final drag = await tester.startGesture(
      _at(250),
      kind: PointerDeviceKind.mouse,
    );
    for (final x in <double>[300, 350, 400, 450]) {
      await drag.moveTo(_at(x));
      await tester.pump(const Duration(milliseconds: 50));
    }
    await tester.pump(_rest);
    await drag.moveTo(_at(500));
    await tester.pump(const Duration(milliseconds: 50));
    await drag.up();
    await tester.pump(_rest);

    expect(seeks, const [
      Duration(hours: 1, minutes: 30), // the press
      Duration(hours: 2, minutes: 42), // the rest at 450
      Duration(hours: 3), // the release at 500
    ]);
  });

  testWidgets('a drag ending where it rested does not seek there again', (
    tester,
  ) async {
    final seeks = await pumpBar(tester);

    final drag = await tester.startGesture(
      _at(250),
      kind: PointerDeviceKind.mouse,
    );
    await drag.moveTo(_at(500));
    await tester.pump(_rest);
    await drag.up();
    await tester.pump(_rest);

    expect(seeks, const [Duration(hours: 1, minutes: 30), Duration(hours: 3)]);
  });

  testWidgets('one bar mounted again listens again', (tester) async {
    // As media_kit's controls do: one widget, mounted each time they show.
    final bar = SeekBar(
      positions: Live(
        now: () => const Duration(hours: 1),
        changes: Stream.empty,
      ),
      durations: Live(now: () => _vod, changes: Stream.empty),
      buffers: Live(now: () => Duration.zero, changes: Stream.empty),
      onSeek: (_) {},
    );
    Widget shown({required bool on}) => Directionality(
      textDirection: TextDirection.ltr,
      child: on ? bar : const SizedBox(),
    );

    await tester.pumpWidget(shown(on: true));
    await tester.pumpWidget(shown(on: false));
    await tester.pumpWidget(shown(on: true));
    await tester.pump();

    expect(tester.takeException(), isNull);
  });

  testWidgets('a drag tells where it is, how far it went, and its end', (
    tester,
  ) async {
    final scrubs = <ScrubReading?>[];
    await pumpBar(tester, scrubs: scrubs);

    final drag = await tester.startGesture(_at(250));
    await drag.moveTo(_at(250) - const Offset(0, 80));
    await drag.moveTo(_at(350) - const Offset(0, 80));
    final moved = scrubs.last;
    await drag.up();
    await tester.pump(_rest);

    expect(scrubs.first?.target, const Duration(hours: 1, minutes: 30));
    expect(moved?.target, const Duration(hours: 1, minutes: 33, seconds: 36));
    expect(moved?.offset, const Duration(minutes: 33, seconds: 36));
    expect(moved?.speed, ScrubSpeed.fine);
    expect(scrubs.last, isNull);
  });

  test('a signed clock reads +m:ss or −h:mm:ss', () {
    expect(signedClock(const Duration(minutes: 2, seconds: 15)), '+2:15');
    expect(signedClock(const Duration(hours: -1, minutes: -2)), '−1:02:00');
  });

  test('a clock reads h:mm:ss, or m:ss under an hour', () {
    expect(clock(const Duration(minutes: 4, seconds: 5)), '4:05');
    expect(clock(const Duration(hours: 2, minutes: 3, seconds: 4)), '2:03:04');
  });
}
