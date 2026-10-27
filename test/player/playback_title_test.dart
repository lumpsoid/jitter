import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/playback_title.dart';
import 'package:jitter/src/bindings/bindings.dart';

Future<void> pumpTitle(WidgetTester tester, PlaybackDetails? details) =>
    tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          appBar: AppBar(
            title: PlaybackTitle(title: 'xQc', details: details),
          ),
        ),
      ),
    );

void main() {
  testWidgets('shows the game under the title', (tester) async {
    await pumpTitle(tester, const PlaybackDetails(game: 'Grand Theft Auto V'));

    expect(find.text('xQc'), findsOneWidget);
    expect(find.text('Grand Theft Auto V'), findsOneWidget);
  });

  testWidgets("shows a VOD's game and streamed day under the title", (
    tester,
  ) async {
    await pumpTitle(
      tester,
      const PlaybackDetails(game: 'Just Chatting', streamed: 'Oct 24'),
    );

    expect(find.text('xQc'), findsOneWidget);
    expect(find.text('Just Chatting · Oct 24'), findsOneWidget);
  });

  testWidgets('shows the streamed day alone without a game', (tester) async {
    await pumpTitle(tester, const PlaybackDetails(streamed: 'Dec 31, 2025'));

    expect(find.text('xQc'), findsOneWidget);
    expect(find.text('Dec 31, 2025'), findsOneWidget);
  });

  testWidgets('shows the title alone without details to show', (tester) async {
    await pumpTitle(tester, const PlaybackDetails());

    expect(find.text('xQc'), findsOneWidget);
    expect(find.byType(Column), findsNothing);
  });

  testWidgets('shows the title alone before playback is ready', (tester) async {
    await pumpTitle(tester, null);

    expect(find.text('xQc'), findsOneWidget);
    expect(find.byType(Column), findsNothing);
  });
}
