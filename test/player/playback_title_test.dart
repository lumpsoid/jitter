import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/playback_title.dart';
import 'package:jitter/src/bindings/bindings.dart';

Future<void> pumpTitle(
  WidgetTester tester,
  PlaybackDetails? details, {
  ValueChanged<String>? onOpenChannel,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      appBar: AppBar(
        title: PlaybackTitle(
          title: 'xQc',
          details: details,
          onOpenChannel: onOpenChannel ?? (_) {},
        ),
      ),
    ),
  ),
);

const _owner = PlaybackChannel(login: 'xqc', displayName: 'xQc_Owner');

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

  testWidgets('marks a live stream and shows its viewers, uptime and game', (
    tester,
  ) async {
    await pumpTitle(
      tester,
      const PlaybackDetails(
        game: 'Just Chatting',
        live: PlaybackLive(viewers: '1.2K viewers', uptime: '2h 15m'),
      ),
    );

    expect(find.text('LIVE'), findsOneWidget);
    expect(find.text('1.2K viewers · 2h 15m · Just Chatting'), findsOneWidget);
  });

  testWidgets('a live stream without an uptime shows its viewers', (
    tester,
  ) async {
    await pumpTitle(
      tester,
      const PlaybackDetails(live: PlaybackLive(viewers: '950 viewers')),
    );

    expect(find.text('LIVE'), findsOneWidget);
    expect(find.text('950 viewers'), findsOneWidget);
  });

  testWidgets('a VOD has no live badge', (tester) async {
    await pumpTitle(
      tester,
      const PlaybackDetails(game: 'Just Chatting', streamed: 'Oct 24'),
    );

    expect(find.text('LIVE'), findsNothing);
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

  testWidgets(
    "shows the channel's name with its avatar in place of the title",
    (tester) async {
      await pumpTitle(
        tester,
        const PlaybackDetails(
          channel: _owner,
          game: 'Just Chatting',
          streamed: 'Oct 24',
        ),
      );

      expect(find.text('xQc_Owner'), findsOneWidget);
      expect(find.text('xQc'), findsNothing);
      expect(find.text('Just Chatting · Oct 24'), findsOneWidget);
      expect(find.byType(CircleAvatar), findsOneWidget);
    },
  );

  testWidgets('tapping the channel opens it by login', (tester) async {
    final opened = <String>[];
    await pumpTitle(
      tester,
      const PlaybackDetails(channel: _owner),
      onOpenChannel: opened.add,
    );

    await tester.tap(find.text('xQc_Owner'));
    expect(opened, ['xqc']);

    await tester.tap(find.byType(CircleAvatar));
    expect(opened, ['xqc', 'xqc']);
  });

  testWidgets('the title is not tappable before the channel is known', (
    tester,
  ) async {
    await pumpTitle(tester, const PlaybackDetails(game: 'Just Chatting'));

    expect(find.byType(InkWell), findsNothing);
    expect(find.byType(CircleAvatar), findsNothing);
  });
}
