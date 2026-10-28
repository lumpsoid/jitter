import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/now_playing.dart';
import 'package:jitter/src/bindings/bindings.dart';

const _chapters = [
  PlaybackChapter(game: 'Just Chatting', start: '0:00', current: false),
  PlaybackChapter(game: 'Grand Theft Auto V', start: '1:23:45', current: true),
];

/// The strip, [width] wide on a screen as wide; the chapters gone to land
/// in [goneTo].
Future<void> pumpStrip(
  WidgetTester tester, {
  String? title,
  String? game,
  List<PlaybackChapter> chapters = const [],
  List<int>? goneTo,
  double width = 400,
}) {
  tester.view.physicalSize = Size(width, 800);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  return tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: Align(
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: width,
            child: NowPlaying(
              title: title,
              game: game,
              chapters: chapters,
              onGoToChapter: (index) => goneTo?.add(index),
            ),
          ),
        ),
      ),
    ),
  );
}

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

  testWidgets('a VOD without chapters has no chapter button', (tester) async {
    await pumpStrip(tester, game: 'Just Chatting');

    expect(find.byIcon(Icons.arrow_drop_down), findsNothing);
    expect(find.byType(InkWell), findsNothing);
  });

  testWidgets('the chapter button keeps the strip its height', (tester) async {
    await pumpStrip(tester, game: 'Just Chatting');
    final plain = stripHeight(tester);

    await pumpStrip(tester, game: 'Just Chatting', chapters: _chapters);
    expect(find.byIcon(Icons.arrow_drop_down), findsOneWidget);
    expect(stripHeight(tester), plain);
  });

  testWidgets('the whole strip is the chapter button, a thumb high', (
    tester,
  ) async {
    await pumpStrip(
      tester,
      title: 'GTA RP',
      game: 'Just Chatting',
      chapters: _chapters,
    );

    final target = tester.getSize(find.byType(InkWell));
    expect(target.height, greaterThanOrEqualTo(kMinInteractiveDimension));
    expect(target.width, 400);
  });

  testWidgets('on a narrow screen the chapters open as a sheet', (
    tester,
  ) async {
    final goneTo = <int>[];
    await pumpStrip(
      tester,
      title: 'GTA RP',
      game: 'Grand Theft Auto V',
      chapters: _chapters,
      goneTo: goneTo,
    );

    await tester.tap(find.text('GTA RP'));
    await tester.pumpAndSettle();
    expect(find.byType(BottomSheet), findsOneWidget);
    expect(find.text('1:23:45'), findsOneWidget);
    final current = find.ancestor(
      of: find.byIcon(Icons.check),
      matching: find.byType(ListTile),
    );
    expect(
      find.descendant(of: current, matching: find.text('Grand Theft Auto V')),
      findsOneWidget,
    );

    await tester.tap(find.text('0:00'));
    await tester.pumpAndSettle();
    expect(goneTo, [0]);
    expect(find.byType(BottomSheet), findsNothing);
  });

  testWidgets('on a wide screen the chapters open as a menu', (tester) async {
    final goneTo = <int>[];
    await pumpStrip(
      tester,
      game: 'Just Chatting',
      chapters: _chapters,
      goneTo: goneTo,
      width: 900,
    );

    await tester.tap(find.text('Just Chatting'));
    await tester.pumpAndSettle();
    expect(find.byType(BottomSheet), findsNothing);
    expect(find.text('1:23:45'), findsOneWidget);

    await tester.tap(find.text('1:23:45'));
    await tester.pumpAndSettle();
    expect(goneTo, [1]);
    expect(find.text('1:23:45'), findsNothing);
  });
}
