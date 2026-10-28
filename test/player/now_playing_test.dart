import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/now_playing.dart';

Future<void> pumpStrip(WidgetTester tester, {String? title, String? game}) =>
    tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Align(
            alignment: Alignment.topLeft,
            child: SizedBox(
              width: 400,
              child: NowPlaying(title: title, game: game),
            ),
          ),
        ),
      ),
    );

double stripHeight(WidgetTester tester) =>
    tester.getSize(find.byType(NowPlaying)).height;

void main() {
  testWidgets('shows the title with the game under it', (tester) async {
    await pumpStrip(tester, title: 'GTA RP', game: 'Grand Theft Auto V');

    final title = tester.getRect(find.text('GTA RP'));
    final game = tester.getRect(find.text('Grand Theft Auto V'));
    expect(game.top, greaterThanOrEqualTo(title.bottom));
  });

  testWidgets('cuts a long title to a line, whole on hover', (tester) async {
    final long = List.filled(80, 'word').join(' ');
    await pumpStrip(tester, title: long);

    final text = tester.widget<Text>(find.text(long));
    expect(text.maxLines, 1);
    expect(text.overflow, TextOverflow.ellipsis);
    expect(find.byTooltip(long), findsOneWidget);
  });

  testWidgets('keeps its height from loading to loaded', (tester) async {
    await pumpStrip(tester);
    final loading = stripHeight(tester);
    expect(loading, greaterThan(0));

    await pumpStrip(tester, game: 'Just Chatting');
    expect(stripHeight(tester), loading);

    await pumpStrip(
      tester,
      title: List.filled(80, 'word').join(' '),
      game: 'Just Chatting',
    );
    expect(stripHeight(tester), loading);
  });

  testWidgets('has no tooltip without a title', (tester) async {
    await pumpStrip(tester, game: 'Just Chatting');

    expect(find.text('Just Chatting'), findsOneWidget);
    expect(find.byType(Tooltip), findsNothing);
  });
}
