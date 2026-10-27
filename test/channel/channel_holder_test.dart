import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/channel/channel_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';

import 'fixtures.dart';

void main() {
  late ChannelHolder holder;
  late int notifications;

  List<String> ids() => switch (holder.vods) {
    VodsStateLoaded(:final vods) => [for (final v in vods) v.id],
    _ => throw StateError('not loaded: ${holder.vods}'),
  };

  setUp(() {
    holder = ChannelHolder();
    notifications = 0;
    holder.addListener(() => notifications++);
  });

  test('holds nothing until Rust answers', () {
    expect(holder.header, isNull);
    expect(holder.vods, isNull);
  });

  test('a header replaces the one before', () {
    holder
      ..applyHeader(const ChannelUpdateLoading(login: 'xqc'))
      ..applyHeader(found());

    expect(
      holder.header,
      isA<ChannelUpdateFound>().having((h) => h.displayName, 'name', 'xQc'),
    );
    expect(notifications, 2);
  });

  test('a not found header is kept', () {
    holder.applyHeader(const ChannelUpdateNotFound(login: 'nobody'));

    expect(
      holder.header,
      isA<ChannelUpdateNotFound>().having((h) => h.login, 'login', 'nobody'),
    );
  });

  test('a VODs snapshot replaces the list', () {
    holder
      ..applyVods(loadedVods(['1', '2']))
      ..applyVods(const VodsUpdateSnapshot(state: VodsStateLoading()));

    expect(holder.vods, isA<VodsStateLoading>());
    expect(notifications, 2);
  });

  test('an append adds its VODs after the loaded ones', () {
    holder
      ..applyVods(loadedVods(['1', '2']))
      ..applyVods(const VodsUpdateAppended(vods: [], more: VodsMoreLoading()));
    expect(ids(), ['1', '2']);
    expect((holder.vods! as VodsStateLoaded).more, isA<VodsMoreLoading>());

    holder.applyVods(
      VodsUpdateAppended(
        vods: [vod('3', progress: 0.5)],
        more: const VodsMoreIdle(),
      ),
    );

    expect(ids(), ['1', '2', '3']);
    final loaded = holder.vods! as VodsStateLoaded;
    expect(loaded.more, isA<VodsMoreIdle>());
    expect(loaded.vods.last.progress, 0.5);
  });

  test('a failed next page keeps the loaded VODs', () {
    holder
      ..applyVods(loadedVods(['1']))
      ..applyVods(
        const VodsUpdateAppended(
          vods: [],
          more: VodsMoreFailed(message: 'timed out'),
        ),
      );

    expect(ids(), ['1']);
    expect(
      (holder.vods! as VodsStateLoaded).more,
      isA<VodsMoreFailed>().having((m) => m.message, 'message', 'timed out'),
    );
  });

  test('an append to a list that is not loaded is ignored', () {
    holder
      ..applyVods(const VodsUpdateSnapshot(state: VodsStateLoading()))
      ..applyVods(
        VodsUpdateAppended(vods: [vod('1')], more: const VodsMoreIdle()),
      );

    expect(holder.vods, isA<VodsStateLoading>());
  });
}
