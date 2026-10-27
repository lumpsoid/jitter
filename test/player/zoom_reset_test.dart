import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/video_zoom.dart';
import 'package:jitter/player/zoom_reset.dart';

void main() {
  testWidgets('shows how far it is zoomed, and resets on a tap', (
    tester,
  ) async {
    final zoom = ValueNotifier(VideoZoom.none);
    addTearDown(zoom.dispose);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: ZoomReset(zoom: zoom)),
      ),
    );

    expect(find.byType(FilledButton), findsNothing);

    zoom.value = VideoZoom.none.zoomBy(2);
    await tester.pump();

    expect(find.text('2.0×'), findsOneWidget);

    await tester.tap(find.byType(FilledButton));
    await tester.pump();

    expect(zoom.value, VideoZoom.none);
    expect(find.byType(FilledButton), findsNothing);
  });
}
