import 'package:jitter/src/bindings/bindings.dart';

CategoryTile tile(String id) => CategoryTile(
  id: id,
  name: 'Game $id',
  boxArtUrl: 'https://cdn/$id-285x380.jpg',
  viewersLabel: '45.6K viewers',
);

CategoriesUpdate loadedCategories(
  List<String> ids, {
  CategoriesMore more = const CategoriesMoreIdle(),
}) => CategoriesUpdateSnapshot(
  state: CategoriesStateLoaded(
    categories: [for (final id in ids) tile(id)],
    more: more,
  ),
);

FavoritesCategoryRow row(String id, List<String> channels) =>
    FavoritesCategoryRow(id: id, name: 'Game $id', channels: channels);
