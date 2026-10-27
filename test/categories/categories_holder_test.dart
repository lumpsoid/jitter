import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/categories/categories_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';

import 'fixtures.dart';

void main() {
  late CategoriesHolder holder;
  late int notifications;

  List<String> ids() => switch (holder.state) {
    CategoriesStateLoaded(:final categories) => [
      for (final c in categories) c.id,
    ],
    _ => throw StateError('not loaded: ${holder.state}'),
  };

  setUp(() {
    holder = CategoriesHolder();
    notifications = 0;
    holder.addListener(() => notifications++);
  });

  test('holds nothing until the first snapshot', () {
    expect(holder.state, isNull);
    expect(holder.fromYourChannels, isEmpty);
  });

  test('an append adds its categories after the loaded ones', () {
    holder
      ..apply(loadedCategories(['a', 'b']))
      ..apply(
        CategoriesUpdateAppended(
          categories: [tile('c')],
          more: const CategoriesMoreLoading(),
        ),
      );

    expect(ids(), ['a', 'b', 'c']);
    expect(
      (holder.state! as CategoriesStateLoaded).more,
      isA<CategoriesMoreLoading>(),
    );
    expect(notifications, 2);
  });

  test('an append before a loaded list is ignored', () {
    holder
      ..apply(const CategoriesUpdateSnapshot(state: CategoriesStateLoading()))
      ..apply(
        CategoriesUpdateAppended(
          categories: [tile('a')],
          more: const CategoriesMoreIdle(),
        ),
      );

    expect(holder.state, isA<CategoriesStateLoading>());
  });

  test('the strip is replaced on its own, leaving the list alone', () {
    holder
      ..apply(loadedCategories(['a']))
      ..apply(
        CategoriesUpdateFromYourChannels(
          categories: [
            row('1', ['Lirik', 'Vinesauce']),
          ],
        ),
      );

    expect(ids(), ['a']);
    expect([for (final r in holder.fromYourChannels) r.id], ['1']);

    holder.apply(const CategoriesUpdateFromYourChannels(categories: []));

    expect(holder.fromYourChannels, isEmpty);
    expect(ids(), ['a']);
  });
}
