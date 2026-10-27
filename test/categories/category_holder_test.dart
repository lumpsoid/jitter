import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/categories/category_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';

import '../top/fixtures.dart';

void main() {
  late CategoryHolder holder;

  setUp(() => holder = CategoryHolder());

  CategoryStateLoaded loaded() => holder.state! as CategoryStateLoaded;

  test('holds nothing until the first snapshot', () {
    expect(holder.state, isNull);
  });

  test('an append adds streams after the loaded ones, pinned ones stay', () {
    holder
      ..apply(
        CategoryUpdateSnapshot(
          state: CategoryStateLoaded(
            favorites: [card('fav')],
            streams: [card('a')],
            more: const CategoryMoreIdle(),
          ),
        ),
      )
      ..apply(
        CategoryUpdateAppended(
          streams: [card('b')],
          more: const CategoryMoreIdle(),
        ),
      );

    expect([for (final s in loaded().favorites) s.id], ['fav']);
    expect([for (final s in loaded().streams) s.id], ['a', 'b']);
  });

  test('an append before a loaded list is ignored', () {
    holder
      ..apply(const CategoryUpdateSnapshot(state: CategoryStateLoading()))
      ..apply(
        CategoryUpdateAppended(
          streams: [card('a')],
          more: const CategoryMoreIdle(),
        ),
      );

    expect(holder.state, isA<CategoryStateLoading>());
  });

  test('a snapshot replaces the pinned favorites too', () {
    holder
      ..apply(
        CategoryUpdateSnapshot(
          state: CategoryStateLoaded(
            favorites: [card('fav')],
            streams: [card('a')],
            more: const CategoryMoreIdle(),
          ),
        ),
      )
      ..apply(
        CategoryUpdateSnapshot(
          state: CategoryStateLoaded(
            favorites: const [],
            streams: [card('fav'), card('a')],
            more: const CategoryMoreIdle(),
          ),
        ),
      );

    expect(loaded().favorites, isEmpty);
    expect([for (final s in loaded().streams) s.id], ['fav', 'a']);
  });
}
