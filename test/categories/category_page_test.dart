import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/categories/category_holder.dart';
import 'package:jitter/categories/category_page.dart';
import 'package:jitter/favorites/favorites_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';

import '../top/fixtures.dart';

Future<void> pumpPage(
  WidgetTester tester,
  CategoryHolder category, {
  ValueChanged<String>? onOpenStream,
}) async {
  // Narrower than two cards: one column, so list order is top to bottom.
  tester.view.physicalSize = const Size(340, 2000);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    MaterialApp(
      home: CategoryPage(
        name: 'Elden Ring',
        category: category,
        favorites: FavoritesHolder(),
        onOpenStream: onOpenStream,
      ),
    ),
  );
}

void main() {
  testWidgets('spins under its title until the first page', (tester) async {
    await pumpPage(tester, CategoryHolder());

    expect(find.text('Elden Ring'), findsOneWidget);
    expect(find.byType(CircularProgressIndicator), findsOneWidget);
  });

  testWidgets('a failed first page shows its message with Retry', (
    tester,
  ) async {
    final category = CategoryHolder()
      ..apply(
        const CategoryUpdateSnapshot(
          state: CategoryStateFailed(message: 'offline'),
        ),
      );
    await pumpPage(tester, category);

    expect(find.text('offline'), findsOneWidget);
    expect(find.text('Retry'), findsOneWidget);
  });

  testWidgets('pinned favorites come before the other streams', (tester) async {
    final category = CategoryHolder()
      ..apply(
        CategoryUpdateSnapshot(
          state: CategoryStateLoaded(
            favorites: [card('fav')],
            streams: [card('a')],
            more: const CategoryMoreIdle(),
          ),
        ),
      );
    await pumpPage(tester, category);

    final fav = tester.getTopLeft(find.text('Title fav'));
    final other = tester.getTopLeft(find.text('Title a'));
    expect(fav.dy, lessThan(other.dy));
  });

  testWidgets('tapping a stream hands on its login', (tester) async {
    final opened = <String>[];
    final category = CategoryHolder()
      ..apply(
        CategoryUpdateSnapshot(
          state: CategoryStateLoaded(
            favorites: const [],
            streams: [card('a')],
            more: const CategoryMoreIdle(),
          ),
        ),
      );
    await pumpPage(tester, category, onOpenStream: opened.add);
    await tester.tap(find.text('Title a'));

    expect(opened, ['login_a']);
  });
}
