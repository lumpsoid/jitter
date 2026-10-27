import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/favorites/favorites_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';

FavoriteRow row(String id, String name, {FavoriteStatus? status}) =>
    FavoriteRow(
      channelId: id,
      login: name.toLowerCase(),
      displayName: name,
      status: status ?? const FavoriteStatusOffline(),
    );

void main() {
  test('has no list until the first snapshot', () {
    final holder = FavoritesHolder();

    expect(holder.favorites, isNull);
    expect(holder.isFavorite('1'), isFalse);
  });

  test('a snapshot replaces the list, in order, and notifies', () {
    final holder = FavoritesHolder();
    var notified = 0;
    holder
      ..addListener(() => notified++)
      ..apply(FavoritesUpdateSnapshot(favorites: [row('1', 'Alpha')]))
      ..apply(
        FavoritesUpdateSnapshot(
          favorites: [
            row(
              '2',
              'Beta',
              status: const FavoriteStatusLive(
                viewersLabel: '42 viewers',
                title: 'speedrun',
              ),
            ),
            row('1', 'Alpha'),
          ],
        ),
      );

    expect(holder.favorites?.map((f) => f.displayName), ['Beta', 'Alpha']);
    expect(notified, 2);
  });

  test('isFavorite reads the list by channel id', () {
    final holder = FavoritesHolder()
      ..apply(FavoritesUpdateSnapshot(favorites: [row('1', 'Alpha')]));

    expect(holder.isFavorite('1'), isTrue);
    expect(holder.isFavorite('2'), isFalse);

    holder.apply(const FavoritesUpdateSnapshot(favorites: []));

    expect(holder.isFavorite('1'), isFalse);
  });

  test('a failure goes out once each, list untouched', () async {
    final holder = FavoritesHolder()
      ..apply(FavoritesUpdateSnapshot(favorites: [row('1', 'Alpha')]));
    final messages = <String>[];
    holder.failures.listen(messages.add);
    var notified = 0;
    holder
      ..addListener(() => notified++)
      ..apply(const FavoritesUpdateFailed(message: "Couldn't add xqc"))
      ..apply(const FavoritesUpdateFailed(message: "Couldn't add xqc"));

    expect(messages, ["Couldn't add xqc", "Couldn't add xqc"]);
    expect(holder.favorites?.map((f) => f.channelId), ['1']);
    expect(notified, 0);
  });
}
