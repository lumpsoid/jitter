import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/favorites/favorites_holder.dart';
import 'package:jitter/favorites/favorites_page.dart';
import 'package:jitter/src/bindings/bindings.dart';

Future<void> pumpPage(
  WidgetTester tester,
  FavoritesHolder holder, {
  void Function(String login)? onOpenChannel,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: FavoritesPage(
        favorites: holder,
        onOpenChannel: onOpenChannel ?? (_) {},
      ),
    ),
  ),
);

void main() {
  testWidgets('spins until the first snapshot', (tester) async {
    await pumpPage(tester, FavoritesHolder());

    expect(find.byType(CircularProgressIndicator), findsOneWidget);
  });

  testWidgets('shows live and offline favorites', (tester) async {
    final holder = FavoritesHolder()
      ..apply(
        const FavoritesUpdateSnapshot(
          favorites: [
            FavoriteRow(
              channelId: '2',
              login: 'beta',
              displayName: 'Beta',
              status: FavoriteStatusLive(
                viewersLabel: '42 viewers',
                title: 'speedrun',
              ),
            ),
            FavoriteRow(
              channelId: '1',
              login: 'alpha',
              displayName: 'Alpha',
              status: FavoriteStatusOffline(),
            ),
          ],
        ),
      );
    final opened = <String>[];
    await pumpPage(tester, holder, onOpenChannel: opened.add);

    expect(find.text('Beta'), findsOneWidget);
    expect(find.text('42 viewers · speedrun'), findsOneWidget);
    expect(find.text('LIVE'), findsOneWidget);
    expect(find.text('Offline'), findsOneWidget);
    expect(find.byIcon(Icons.star), findsNWidgets(2));

    await tester.tap(find.text('Alpha'));
    expect(opened, ['alpha']);
  });

  testWidgets('shows a failure as a snackbar', (tester) async {
    final holder = FavoritesHolder()
      ..apply(const FavoritesUpdateSnapshot(favorites: []));
    await pumpPage(tester, holder);

    holder.apply(const FavoritesUpdateFailed(message: 'Storage failed'));
    await tester.pump();

    expect(find.text('Storage failed'), findsOneWidget);
  });
}
