import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/video_zoom.dart';

void main() {
  test('none shows the whole picture', () {
    expect(VideoZoom.none.zoomed, isFalse);
    expect(VideoZoom.none.center, const Offset(0.5, 0.5));
  });

  test('zooming about the middle keeps the middle', () {
    final zoom = VideoZoom.none.zoomBy(2);

    expect(zoom.zoomed, isTrue);
    expect(zoom.scale, 2);
    expect(zoom.center, const Offset(0.5, 0.5));
  });

  test('zooming about a point keeps that point of the picture under it', () {
    final zoom = VideoZoom.none.zoomBy(2, focus: const Offset(0.75, 0.5));

    // The frame's x = 0.75 shows the picture's x = 0.5 + 0.25 / 2 = 0.625.
    expect(zoom.center.dx + (0.75 - 0.5) / zoom.scale, 0.75);
    expect(zoom.center.dy, 0.5);
  });

  test('zooming never shows past the picture edges', () {
    final zoom = VideoZoom.none.zoomBy(2, focus: Offset.zero).zoomBy(0.75);

    // At 1.5x the frame shows a third of the picture each side of center.
    expect(zoom.scale, 1.5);
    expect(zoom.center.dx, closeTo(1 / 3, 1e-9));
    expect(zoom.center.dy, closeTo(1 / 3, 1e-9));
  });

  test('zoom stays between the whole picture and eight times it', () {
    expect(VideoZoom.none.zoomBy(0.5), VideoZoom.none);
    expect(VideoZoom.none.zoomBy(100).scale, 8);
  });

  test('zooming back out to the whole picture recenters it', () {
    final zoom = VideoZoom.none
        .zoomBy(4, focus: const Offset(0.9, 0.1))
        .zoomBy(0.25);

    expect(zoom, VideoZoom.none);
  });

  test('a drag moves the picture with it, slower the further in', () {
    final zoom = VideoZoom.none.zoomBy(4).panBy(const Offset(0.2, -0.1));

    expect(zoom.center.dx, closeTo(0.45, 1e-9));
    expect(zoom.center.dy, closeTo(0.525, 1e-9));
  });

  test('a drag stops at the picture edges', () {
    final zoom = VideoZoom.none.zoomBy(2).panBy(const Offset(5, -5));

    expect(zoom.center, const Offset(0.25, 0.75));
  });

  test('a drag does nothing to the whole picture', () {
    expect(VideoZoom.none.panBy(const Offset(0.3, 0.3)), VideoZoom.none);
  });
}
