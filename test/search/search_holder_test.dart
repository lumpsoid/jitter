import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/search/search_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';

import '../categories/fixtures.dart';
import 'fixtures.dart';

void main() {
  late SearchHolder holder;

  setUp(() => holder = SearchHolder());

  List<String> logins() => switch (holder.channels) {
    SearchChannelsStateLoaded(:final channels) => [
      for (final c in channels) c.login,
    ],
    _ => const [],
  };

  List<String> ids() => switch (holder.categories) {
    SearchCategoriesStateLoaded(:final categories) => [
      for (final c in categories) c.id,
    ],
    _ => const [],
  };

  test('both lists are idle until Rust sends one', () {
    expect(holder.channels, isA<SearchChannelsStateIdle>());
    expect(holder.categories, isA<SearchCategoriesStateIdle>());
  });

  test('a snapshot replaces only its own list', () {
    holder
      ..apply(searchedChannels(['a']))
      ..apply(searchedCategories(['1']))
      ..apply(searchedChannels(['b']));

    expect(logins(), ['b']);
    expect(ids(), ['1']);
  });

  test('appends add results after the loaded ones', () {
    holder
      ..apply(searchedChannels(['a'], more: const SearchMoreLoading()))
      ..apply(
        SearchUpdateChannelsAppended(
          channels: [channelResult('b')],
          more: const SearchMoreIdle(),
        ),
      )
      ..apply(searchedCategories(['1']))
      ..apply(
        SearchUpdateCategoriesAppended(
          categories: [tile('2')],
          more: const SearchMoreFailed(message: 'timed out'),
        ),
      );

    expect(logins(), ['a', 'b']);
    expect(ids(), ['1', '2']);
    final categories = holder.categories as SearchCategoriesStateLoaded;
    expect(categories.more, isA<SearchMoreFailed>());
  });

  test('an append to a list that is not loaded is ignored', () {
    holder.apply(
      SearchUpdateChannelsAppended(
        channels: [channelResult('a')],
        more: const SearchMoreIdle(),
      ),
    );

    expect(holder.channels, isA<SearchChannelsStateIdle>());
  });

  test('a cleared search goes back to idle', () {
    holder
      ..apply(searchedChannels(['a']))
      ..apply(const SearchUpdateChannels(state: SearchChannelsStateIdle()));

    expect(holder.channels, isA<SearchChannelsStateIdle>());
  });

  test('every update notifies', () {
    var notified = 0;
    holder
      ..addListener(() => notified++)
      ..apply(searchedChannels(['a']))
      ..apply(searchedCategories(['1']));

    expect(notified, 2);
  });
}
