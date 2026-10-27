import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/favorites/favorites_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:jitter/top/browse_holder.dart';
import 'package:jitter/top/top_page.dart';

import 'fixtures.dart';

Future<void> pumpPage(
  WidgetTester tester,
  BrowseHolder browse, {
  ValueChanged<String>? onOpenStream,
  FavoritesHolder? favorites,
}) async {
  tester.view.physicalSize = const Size(400, 2000);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: TopPage(
          browse: browse,
          favorites: favorites ?? FavoritesHolder(),
          onOpenStream: onOpenStream,
        ),
      ),
    ),
  );
}

void main() {
  testWidgets('spins until the first page', (tester) async {
    await pumpPage(tester, BrowseHolder());

    expect(find.byType(CircularProgressIndicator), findsOneWidget);
  });

  testWidgets('a failed first page shows its message with Retry', (
    tester,
  ) async {
    final browse = BrowseHolder()
      ..apply(
        const BrowseUpdateSnapshot(
          state: BrowseStateFailed(message: 'offline'),
        ),
      );
    await pumpPage(tester, browse);

    expect(find.text('offline'), findsOneWidget);
    expect(find.text('Retry'), findsOneWidget);
  });

  testWidgets('a failed next page keeps the cards above its Retry', (
    tester,
  ) async {
    final browse = BrowseHolder()
      ..apply(loaded(['a']))
      ..apply(
        const BrowseUpdateAppended(
          streams: [],
          more: BrowseMoreFailed(message: 'timed out'),
        ),
      );
    await pumpPage(tester, browse);

    expect(find.text('Title a'), findsOneWidget);
    expect(find.text('12.3K viewers'), findsOneWidget);
    expect(find.text('timed out'), findsOneWidget);
    expect(find.text('Retry'), findsOneWidget);
  });

  testWidgets('tapping a card hands on its login', (tester) async {
    final opened = <String>[];
    final browse = BrowseHolder()
      ..apply(
        BrowseUpdateSnapshot(
          state: BrowseStateLoaded(
            streams: [card('a')],
            more: const BrowseMoreIdle(),
          ),
        ),
      );
    await pumpPage(tester, browse, onOpenStream: opened.add);
    await tester.tap(find.text('Title a'));

    expect(opened, ['login_a']);
  });

  testWidgets("a card's star is filled while its channel is a favorite", (
    tester,
  ) async {
    final favorites = FavoritesHolder()
      ..apply(
        const FavoritesUpdateSnapshot(
          favorites: [
            FavoriteRow(
              channelId: 'channel_a',
              login: 'login_a',
              displayName: 'Name a',
              status: FavoriteStatusOffline(),
            ),
          ],
        ),
      );
    final browse = BrowseHolder()..apply(loaded(['a', 'b']));
    await pumpPage(tester, browse, favorites: favorites);

    expect(find.byIcon(Icons.star), findsOneWidget);
    expect(find.byIcon(Icons.star_outline), findsOneWidget);
  });
}
