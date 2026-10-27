import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/video_zoom.dart';
import 'package:jitter/player/zoom_gestures.dart';

/// Pumps the gestures over a 400×200 box at the top-left of the screen,
/// its left half kept from them by [KeepsWheel].
Future<ValueNotifier<VideoZoom>> pumpGestures(WidgetTester tester) async {
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

void main() {
  testWidgets('the wheel zooms in and out about the pointer', (tester) async {
    final zoom = await pumpGestures(tester);
    final pointer = TestPointer(1, PointerDeviceKind.mouse);
    const at = Offset(300, 50);

    await tester.sendEventToBinding(pointer.hover(at));
    await tester.sendEventToBinding(pointer.scroll(const Offset(0, -20)));

    expect(
      zoom.value,
      VideoZoom.none.zoomBy(zoomStep, focus: const Offset(0.75, 0.25)),
    );

    await tester.sendEventToBinding(pointer.scroll(const Offset(0, 20)));

    expect(zoom.value, VideoZoom.none);
  });

  testWidgets('the wheel over a KeepsWheel child does not zoom', (
    tester,
  ) async {
    final zoom = await pumpGestures(tester);
    final pointer = TestPointer(1, PointerDeviceKind.mouse);

    await tester.sendEventToBinding(pointer.hover(const Offset(100, 100)));
    await tester.sendEventToBinding(pointer.scroll(const Offset(0, -20)));

    expect(zoom.value, VideoZoom.none);
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
