import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/favorites/favorites_holder.dart';
import 'package:jitter/favorites/favorites_page.dart';
import 'package:jitter/src/bindings/bindings.dart';

import 'fixtures.dart';

Future<void> pumpPage(
  WidgetTester tester,
  FavoritesHolder holder, {
  void Function(String login)? onOpenChannel,
  void Function(String login)? onOpenLive,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: FavoritesPage(
        favorites: holder,
        onOpenChannel: onOpenChannel ?? (_) {},
        onOpenLive: onOpenLive ?? (_) {},
      ),
    ),
  ),
);

void main() {
  testWidgets('spins until the first snapshot', (tester) async {
    await pumpPage(tester, FavoritesHolder());

    expect(find.byType(CircularProgressIndicator), findsOneWidget);
  });

  testWidgets('says how to add one when there are no favorites', (
    tester,
  ) async {
    final holder = FavoritesHolder()..apply(snapshot());
    await pumpPage(tester, holder);

    expect(
      find.text('No favorites yet. Star a channel to keep it here.'),
      findsOneWidget,
    );
    expect(find.textContaining('Live now'), findsNothing);
  });

  testWidgets('shows live favorites first, then offline ones', (tester) async {
    final holder = FavoritesHolder()
      ..apply(
        snapshot(live: [live('2', 'Beta')], offline: [channel('1', 'Alpha')]),
      );
    await pumpPage(tester, holder);

    expect(find.text('Live now · 1'), findsOneWidget);
    expect(find.text('Offline · 1'), findsOneWidget);
    expect(find.textContaining('Not checked'), findsNothing);
    expect(find.text('42 viewers'), findsOneWidget);
    expect(find.text('3h 12m'), findsOneWidget);
    expect(find.text('speedrun'), findsOneWidget);
    expect(find.byIcon(Icons.star), findsNWidgets(2));
    expect(
      tester.getTopLeft(find.text('Beta')).dy,
      lessThan(tester.getTopLeft(find.text('Alpha')).dy),
    );
  });

  testWidgets('a live card opens the player; its name opens the channel', (
    tester,
  ) async {
    final holder = FavoritesHolder()
      ..apply(snapshot(live: [live('2', 'Beta')]));
    final played = <String>[];
    final opened = <String>[];
    await pumpPage(
      tester,
      holder,
      onOpenLive: played.add,
      onOpenChannel: opened.add,
    );

    expect(find.textContaining('Offline'), findsNothing);

    await tester.tap(find.text('speedrun'));
    await tester.tap(find.text('Beta'));

    expect(played, ['beta']);
    expect(opened, ['beta']);
  });

  testWidgets('an offline channel opens its page', (tester) async {
    final holder = FavoritesHolder()
      ..apply(snapshot(offline: [channel('1', 'Alpha')]));
    final opened = <String>[];
    await pumpPage(tester, holder, onOpenChannel: opened.add);

    expect(find.textContaining('Live now'), findsNothing);

    await tester.tap(find.text('Alpha'));

    expect(opened, ['alpha']);
  });

  testWidgets('unchecked favorites are not shown as offline', (tester) async {
    final holder = FavoritesHolder()
      ..apply(snapshot(unchecked: [channel('1', 'Alpha')]));
    await pumpPage(tester, holder);

    expect(find.text('Not checked yet · 1'), findsOneWidget);
    expect(find.textContaining('Offline'), findsNothing);
    expect(find.text('Alpha'), findsOneWidget);
    expect(find.byIcon(Icons.star), findsOneWidget);
  });

  testWidgets('shows a failure as a snackbar', (tester) async {
    final holder = FavoritesHolder()..apply(snapshot());
    await pumpPage(tester, holder);

    holder.apply(const FavoritesUpdateFailed(message: 'Storage failed'));
    await tester.pump();

    expect(find.text('Storage failed'), findsOneWidget);
  });
}
