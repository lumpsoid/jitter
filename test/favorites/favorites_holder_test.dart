import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/favorites/favorites_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';

import 'fixtures.dart';

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
      ..apply(snapshot(unchecked: [channel('1', 'Alpha')]))
      ..apply(
        snapshot(live: [live('2', 'Beta')], offline: [channel('1', 'Alpha')]),
      );

    final favorites = holder.favorites;
    expect(favorites?.live.map((f) => f.stream.displayName), ['Beta']);
    expect(favorites?.offline.map((f) => f.displayName), ['Alpha']);
    expect(favorites?.unchecked, isEmpty);
    expect(notified, 2);
  });

  test('isFavorite reads every part of the list by channel id', () {
    final holder = FavoritesHolder()
      ..apply(
        snapshot(
          live: [live('1', 'Alpha')],
          offline: [channel('2', 'Beta')],
          unchecked: [channel('3', 'Gamma')],
        ),
      );

    expect(holder.isFavorite('1'), isTrue);
    expect(holder.isFavorite('2'), isTrue);
    expect(holder.isFavorite('3'), isTrue);
    expect(holder.isFavorite('4'), isFalse);
    expect(holder.isEmpty, isFalse);

    holder.apply(snapshot());

    expect(holder.isFavorite('1'), isFalse);
    expect(holder.isEmpty, isTrue);
  });

  test('a failure goes out once each, list untouched', () async {
    final holder = FavoritesHolder()
      ..apply(snapshot(offline: [channel('1', 'Alpha')]));
    final messages = <String>[];
    holder.failures.listen(messages.add);
    var notified = 0;
    holder
      ..addListener(() => notified++)
      ..apply(const FavoritesUpdateFailed(message: "Couldn't add xqc"))
      ..apply(const FavoritesUpdateFailed(message: "Couldn't add xqc"));

    expect(messages, ["Couldn't add xqc", "Couldn't add xqc"]);
    expect(holder.favorites?.offline.map((f) => f.channelId), ['1']);
    expect(notified, 0);
  });
}
