import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/playback_loading.dart';

void main() {
  testWidgets('loading is a live region labelled for a screen reader', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    await tester.pumpWidget(
      const MaterialApp(home: Scaffold(body: PlaybackLoading())),
    );

    expect(find.byType(CircularProgressIndicator), findsOneWidget);
    expect(
      tester.getSemantics(find.bySemanticsLabel('Loading stream')),
      isSemantics(label: 'Loading stream', isLiveRegion: true),
    );
    semantics.dispose();
  });
}
