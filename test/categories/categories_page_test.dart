import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/categories/categories_holder.dart';
import 'package:jitter/categories/categories_page.dart';
import 'package:jitter/src/bindings/bindings.dart';

import 'fixtures.dart';

Future<void> pumpPage(
  WidgetTester tester,
  CategoriesHolder categories, {
  CategoryOpener? onOpenCategory,
}) async {
  tester.view.physicalSize = const Size(400, 2000);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: CategoriesPage(
          categories: categories,
          onOpenCategory: onOpenCategory,
        ),
      ),
    ),
  );
}

void main() {
  testWidgets('spins until the first page', (tester) async {
    await pumpPage(tester, CategoriesHolder());

    expect(find.byType(CircularProgressIndicator), findsOneWidget);
  });

  testWidgets('a failed first page shows its message with Retry', (
    tester,
  ) async {
    final categories = CategoriesHolder()
      ..apply(
        const CategoriesUpdateSnapshot(
          state: CategoriesStateFailed(message: 'offline'),
        ),
      );
    await pumpPage(tester, categories);

    expect(find.text('offline'), findsOneWidget);
    expect(find.text('Retry'), findsOneWidget);
  });

  testWidgets('a failed next page keeps the tiles above its Retry', (
    tester,
  ) async {
    final categories = CategoriesHolder()
      ..apply(loadedCategories(['a']))
      ..apply(
        const CategoriesUpdateAppended(
          categories: [],
          more: CategoriesMoreFailed(message: 'timed out'),
        ),
      );
    await pumpPage(tester, categories);

    expect(find.text('Game a'), findsOneWidget);
    expect(find.text('45.6K viewers'), findsOneWidget);
    expect(find.text('timed out'), findsOneWidget);
  });

  testWidgets('without live favorites there is no strip', (tester) async {
    final categories = CategoriesHolder()..apply(loadedCategories(['a']));
    await pumpPage(tester, categories);

    expect(find.text('From your channels'), findsNothing);
  });

  testWidgets('the strip names the favorites playing each category', (
    tester,
  ) async {
    final categories = CategoriesHolder()
      ..apply(loadedCategories(['a']))
      ..apply(
        CategoriesUpdateFromYourChannels(
          categories: [
            row('1', ['Lirik', 'Vinesauce']),
          ],
        ),
      );
    await pumpPage(tester, categories);

    expect(find.text('From your channels'), findsOneWidget);
    expect(find.text('Game 1'), findsOneWidget);
    expect(find.text('▸ Lirik, Vinesauce'), findsOneWidget);
  });

  testWidgets('tapping a tile or a strip entry opens its category', (
    tester,
  ) async {
    final opened = <(String, String)>[];
    final categories = CategoriesHolder()
      ..apply(loadedCategories(['a']))
      ..apply(
        CategoriesUpdateFromYourChannels(
          categories: [
            row('1', ['Lirik']),
          ],
        ),
      );
    await pumpPage(
      tester,
      categories,
      onOpenCategory: (id, name) => opened.add((id, name)),
    );
    await tester.tap(find.text('Game a'));
    await tester.tap(find.text('Game 1'));

    expect(opened, [('a', 'Game a'), ('1', 'Game 1')]);
  });
}
