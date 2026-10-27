import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/video_zoom.dart';
import 'package:jitter/player/zoom_gestures.dart';

/// Pumps the gestures over a 400×200 box at the top-left of the screen,
/// its left half kept from them by [KeepsWheel]. Each plain wheel notch
/// they pass on is added to [wheeled].
Future<ValueNotifier<VideoZoom>> pumpGestures(
  WidgetTester tester, {
  List<int>? wheeled,
}) async {
  final zoom = ValueNotifier(VideoZoom.none);
  addTearDown(zoom.dispose);
  await tester.pumpWidget(
    Directionality(
      textDirection: TextDirection.ltr,
      child: Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 400,
          height: 200,
          child: ZoomGestures(
            zoom: zoom,
            onWheel: (notches) => wheeled?.add(notches),
            child: const Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Expanded(
                  child: KeepsWheel(
                    child: ColoredBox(color: Color(0xFF000000)),
                  ),
                ),
                Expanded(child: ColoredBox(color: Color(0xFF000000))),
              ],
            ),
          ),
        ),
      ),
    ),
  );
  return zoom;
}

/// Scrolls one notch up (or down) at [at].
Future<void> wheel(WidgetTester tester, Offset at, {bool up = true}) async {
  final pointer = TestPointer(1, PointerDeviceKind.mouse);
  await tester.sendEventToBinding(pointer.hover(at));
  await tester.sendEventToBinding(pointer.scroll(Offset(0, up ? -20 : 20)));
}

void main() {
  testWidgets('Ctrl and the wheel zoom in and out about the pointer', (
    tester,
  ) async {
    final wheeled = <int>[];
    final zoom = await pumpGestures(tester, wheeled: wheeled);
    const at = Offset(300, 50);

    await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
    await wheel(tester, at);

    expect(
      zoom.value,
      VideoZoom.none.zoomBy(zoomStep, focus: const Offset(0.75, 0.25)),
    );

    await wheel(tester, at, up: false);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);

    expect(zoom.value, VideoZoom.none);
    expect(wheeled, isEmpty);
  });

  testWidgets('the wheel alone is passed on, not zoomed', (tester) async {
    final wheeled = <int>[];
    final zoom = await pumpGestures(tester, wheeled: wheeled);

    await wheel(tester, const Offset(300, 50));
    await wheel(tester, const Offset(300, 50), up: false);

    expect(wheeled, [1, -1]);
    expect(zoom.value, VideoZoom.none);
  });

  testWidgets("the wheel over a KeepsWheel child is the child's alone", (
    tester,
  ) async {
    final wheeled = <int>[];
    final zoom = await pumpGestures(tester, wheeled: wheeled);

    await wheel(tester, const Offset(100, 100));
    await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
    await wheel(tester, const Offset(100, 100));
    await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);

    expect(wheeled, isEmpty);
    expect(zoom.value, VideoZoom.none);
  });

  testWidgets('a touch drag moves the picture under media_kit-like drags', (
    tester,
  ) async {
    final zoom = ValueNotifier(VideoZoom.none.zoomBy(4));
    addTearDown(zoom.dispose);
    await tester.pumpWidget(
      SizedBox(
        width: 400,
        height: 200,
        child: ZoomGestures(
          zoom: zoom,
          onWheel: (_) {},
          // As media_kit's mobile controls: both drags, always on.
          child: GestureDetector(
            onHorizontalDragUpdate: (_) {},
            onVerticalDragUpdate: (_) {},
            child: const ColoredBox(color: Color(0xFF000000)),
          ),
        ),
      ),
    );

    // A touch drag: dragFrom's default.
    await tester.dragFrom(const Offset(200, 100), const Offset(-100, 0));

    expect(zoom.value.center.dx, greaterThan(0.5));
  });

  testWidgets('a pinch zooms without a drag moving it twice', (tester) async {
    final zoom = await pumpGestures(tester);
    final left = await tester.startGesture(const Offset(280, 100), pointer: 1);
    final right = await tester.startGesture(const Offset(320, 100), pointer: 2);
    for (var i = 0; i < 10; i++) {
      await left.moveBy(const Offset(-6, 0));
      await right.moveBy(const Offset(6, 0));
    }
    await left.up();
    await right.up();

    expect(zoom.value.scale, greaterThan(1.5));
    // Spread evenly about x = 300, the picture's x = 0.75 stays under it.
    final under = zoom.value.center.dx + 0.25 / zoom.value.scale;
    expect(under, closeTo(0.75, 0.01));
  });

  testWidgets('a drag moves the zoomed picture', (tester) async {
    final zoom = await pumpGestures(tester);
    zoom.value = VideoZoom.none.zoomBy(4);

    await tester.dragFrom(const Offset(200, 100), const Offset(-100, 0));

    expect(zoom.value.scale, 4);
    expect(zoom.value.center.dx, greaterThan(0.5));
    expect(zoom.value.center.dy, 0.5);
  });
}
