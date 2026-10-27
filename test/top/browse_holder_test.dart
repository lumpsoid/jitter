import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:jitter/top/browse_holder.dart';

import 'fixtures.dart';

void main() {
  late BrowseHolder holder;
  late int notifications;

  List<String> ids() => switch (holder.state) {
    BrowseStateLoaded(:final streams) => [for (final s in streams) s.id],
    _ => throw StateError('not loaded: ${holder.state}'),
  };

  setUp(() {
    holder = BrowseHolder();
    notifications = 0;
    holder.addListener(() => notifications++);
  });

  test('holds nothing until the first snapshot', () {
    expect(holder.state, isNull);
  });

  test('a snapshot replaces the state', () {
    holder
      ..apply(loaded(['a', 'b']))
      ..apply(const BrowseUpdateSnapshot(state: BrowseStateLoading()));

    expect(holder.state, isA<BrowseStateLoading>());
    expect(notifications, 2);
  });

  test('an append adds its streams after the loaded ones', () {
    holder
      ..apply(loaded(['a', 'b']))
      ..apply(
        const BrowseUpdateAppended(streams: [], more: BrowseMoreLoading()),
      );
    expect(ids(), ['a', 'b']);
    expect((holder.state! as BrowseStateLoaded).more, isA<BrowseMoreLoading>());

    holder.apply(
      BrowseUpdateAppended(
        streams: [card('c'), card('d')],
        more: const BrowseMoreIdle(),
      ),
    );

    expect(ids(), ['a', 'b', 'c', 'd']);
    expect((holder.state! as BrowseStateLoaded).more, isA<BrowseMoreIdle>());
    expect(notifications, 3);
  });

  test('a failed next page keeps the loaded streams', () {
    holder
      ..apply(loaded(['a', 'b']))
      ..apply(
        BrowseUpdateAppended(
          streams: [card('c')],
          more: const BrowseMoreIdle(),
        ),
      )
      ..apply(
        const BrowseUpdateAppended(
          streams: [],
          more: BrowseMoreFailed(message: 'timed out'),
        ),
      );

    expect(ids(), ['a', 'b', 'c']);
    expect(
      (holder.state! as BrowseStateLoaded).more,
      isA<BrowseMoreFailed>().having((m) => m.message, 'message', 'timed out'),
    );
  });

  test('a refresh after a failure starts over', () {
    holder
      ..apply(
        const BrowseUpdateSnapshot(
          state: BrowseStateFailed(message: 'offline'),
        ),
      )
      ..apply(const BrowseUpdateSnapshot(state: BrowseStateLoading()))
      ..apply(loaded(['x']));

    expect(ids(), ['x']);
  });

  test('an append before a loaded list is ignored', () {
    holder
      ..apply(const BrowseUpdateSnapshot(state: BrowseStateLoading()))
      ..apply(
        BrowseUpdateAppended(
          streams: [card('a')],
          more: const BrowseMoreIdle(),
        ),
      );

    expect(holder.state, isA<BrowseStateLoading>());
  });
}
