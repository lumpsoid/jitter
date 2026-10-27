import 'package:jitter/src/bindings/bindings.dart';

StreamCard card(String id) => StreamCard(
  id: id,
  channelId: 'channel_$id',
  login: 'login_$id',
  displayName: 'Name $id',
  title: 'Title $id',
  gameName: 'Celeste',
  viewersLabel: '12.3K viewers',
  thumbnailUrl: 'https://cdn/$id-440x248.jpg',
);

BrowseUpdate loaded(
  List<String> ids, {
  BrowseMore more = const BrowseMoreIdle(),
}) => BrowseUpdateSnapshot(
  state: BrowseStateLoaded(
    streams: [for (final id in ids) card(id)],
    more: more,
  ),
);
