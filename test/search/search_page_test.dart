import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/categories/categories_page.dart';
import 'package:jitter/search/search_holder.dart';
import 'package:jitter/search/search_page.dart';
import 'package:jitter/src/bindings/bindings.dart';

import 'fixtures.dart';

Future<void> pumpPage(
  WidgetTester tester,
  SearchHolder search, {
  ValueChanged<String>? onOpenChannel,
  CategoryOpener? onOpenCategory,
}) async {
  tester.view.physicalSize = const Size(400, 2000);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: SearchPage(
          search: search,
          onOpenChannel: onOpenChannel,
          onOpenCategory: onOpenCategory,
        ),
      ),
    ),
  );
}

Future<void> showCategories(WidgetTester tester) async {
  await tester.tap(find.text('Categories'));
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('before a search it shows the field and a hint', (tester) async {
    await pumpPage(tester, SearchHolder());

    expect(find.byType(TextField), findsOneWidget);
    expect(find.text('Channels'), findsOneWidget);
    expect(find.text('Categories'), findsOneWidget);
    expect(find.text('Find channels and categories by name.'), findsOneWidget);
  });

  testWidgets('spins while the first page loads', (tester) async {
    final search = SearchHolder()
      ..apply(const SearchUpdateChannels(state: SearchChannelsStateLoading()));
    await pumpPage(tester, search);

    expect(find.byType(CircularProgressIndicator), findsOneWidget);
  });

  testWidgets('channels show followers and live viewers', (tester) async {
    final search = SearchHolder()
      ..apply(
        SearchUpdateChannels(
          state: SearchChannelsStateLoaded(
            channels: [
              channelResult('lirik'),
              channelResult('lirik_247', liveLabel: '513 viewers'),
            ],
            more: const SearchMoreIdle(),
          ),
        ),
      );
    await pumpPage(tester, search);

    expect(find.text('Name lirik'), findsOneWidget);
    expect(find.text('Name lirik_247'), findsOneWidget);
    expect(find.text('39.9K followers'), findsNWidgets(2));
    expect(find.text('● 513 viewers'), findsOneWidget);
  });

  testWidgets('a failed first page shows its message with Retry', (
    tester,
  ) async {
    final search = SearchHolder()
      ..apply(
        const SearchUpdateChannels(
          state: SearchChannelsStateFailed(message: 'offline'),
        ),
      );
    await pumpPage(tester, search);

    expect(find.text('offline'), findsOneWidget);
    expect(find.text('Retry'), findsOneWidget);
  });

  testWidgets('a failed next page keeps the results above its Retry', (
    tester,
  ) async {
    final search = SearchHolder()
      ..apply(
        searchedChannels([
          'a',
        ], more: const SearchMoreFailed(message: 'timed out')),
      );
    await pumpPage(tester, search);

    expect(find.text('Name a'), findsOneWidget);
    expect(find.text('timed out'), findsOneWidget);
    expect(find.text('Retry'), findsOneWidget);
  });

  testWidgets('no matches say so in each tab', (tester) async {
    final search = SearchHolder()
      ..apply(searchedChannels([]))
      ..apply(searchedCategories([]));
    await pumpPage(tester, search);

    expect(find.text('No channels match.'), findsOneWidget);
    await showCategories(tester);
    expect(find.text('No categories match.'), findsOneWidget);
  });

  testWidgets('categories show as tiles in their tab', (tester) async {
    final search = SearchHolder()..apply(searchedCategories(['1', '2']));
    await pumpPage(tester, search);
    await showCategories(tester);

    expect(find.text('Game 1'), findsOneWidget);
    expect(find.text('Game 2'), findsOneWidget);
    expect(find.text('45.6K viewers'), findsNWidgets(2));
  });

  testWidgets('tapping a channel opens it by login', (tester) async {
    final opened = <String>[];
    final search = SearchHolder()..apply(searchedChannels(['lirik', 'xqc']));
    await pumpPage(tester, search, onOpenChannel: opened.add);
    await tester.tap(find.text('Name xqc'));

    expect(opened, ['xqc']);
  });

  testWidgets('tapping a category opens it by id and name', (tester) async {
    final opened = <(String, String)>[];
    final search = SearchHolder()..apply(searchedCategories(['1', '2']));
    await pumpPage(
      tester,
      search,
      onOpenCategory: (id, name) => opened.add((id, name)),
    );
    await showCategories(tester);
    await tester.tap(find.text('Game 2'));

    expect(opened, [('2', 'Game 2')]);
  });
}
