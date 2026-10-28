import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/swipe_down.dart';
import 'package:jitter/player/video_zoom.dart';

/// Pumps the swipe over a 400×400 box at the top-left of the screen.
/// Returns the number of swipes, and the zoom it reads.
Future<(ValueNotifier<int>, ValueNotifier<VideoZoom>)> pumpSwipe(
  WidgetTester tester,
) async {
  final swipes = ValueNotifier(0);
  final zoom = ValueNotifier(VideoZoom.none);
  addTearDown(swipes.dispose);
  addTearDown(zoom.dispose);
  await tester.pumpWidget(
    Align(
      alignment: Alignment.topLeft,
      child: SizedBox(
        width: 400,
        height: 400,
        child: SwipeDown(
          zoom: zoom,
          onSwipe: () => swipes.value++,
          child: const ColoredBox(color: Color(0xFF000000)),
        ),
      ),
    ),
  );
  return (swipes, zoom);
}

/// Drags from [from] by [by] in steps, then lifts.
Future<void> drag(
  WidgetTester tester,
  Offset from,
  Offset by, {
  PointerDeviceKind kind = PointerDeviceKind.touch,
}) async {
  final gesture = await tester.startGesture(from, kind: kind);
  for (var step = 1; step <= 5; step++) {
    await gesture.moveTo(from + by * (step / 5));
  }
  await gesture.up();
}

void main() {
  testWidgets('a steep swipe down calls it', (tester) async {
    final (swipes, _) = await pumpSwipe(tester);

    await drag(tester, const Offset(200, 50), const Offset(30, 150));

    expect(swipes.value, 1);
  });

  testWidgets('a short, a slanted or an upward swipe does not', (tester) async {
    final (swipes, _) = await pumpSwipe(tester);

    await drag(tester, const Offset(200, 50), const Offset(0, 60));
    await drag(tester, const Offset(100, 50), const Offset(150, 150));
    await drag(tester, const Offset(200, 300), const Offset(0, -150));

    expect(swipes.value, 0);
  });

  testWidgets('a mouse drag does not', (tester) async {
    final (swipes, _) = await pumpSwipe(tester);

    await drag(
      tester,
      const Offset(200, 50),
      const Offset(0, 150),
      kind: PointerDeviceKind.mouse,
    );

    expect(swipes.value, 0);
  });

  testWidgets('zoomed, the drag is the picture', (tester) async {
    final (swipes, zoom) = await pumpSwipe(tester);
    zoom.value = VideoZoom.none.zoomBy(2);

    await drag(tester, const Offset(200, 50), const Offset(0, 150));

    expect(swipes.value, 0);
  });

  testWidgets('a pinch does not, though a finger went down', (tester) async {
    final (swipes, _) = await pumpSwipe(tester);
    final first = await tester.startGesture(const Offset(200, 50));
    final second = await tester.startGesture(const Offset(250, 50));

    await first.moveTo(const Offset(200, 250));
    await second.up();
    await first.up();

    expect(swipes.value, 0);
  });
}
