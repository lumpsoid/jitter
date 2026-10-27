import 'package:jitter/src/bindings/bindings.dart';

FavoriteChannel channel(String id, String name, {String? avatarUrl}) =>
    FavoriteChannel(
      channelId: id,
      login: name.toLowerCase(),
      displayName: name,
      avatarUrl: avatarUrl,
    );

LiveFavorite live(String id, String name) => LiveFavorite(
  stream: StreamCard(
    id: 's$id',
    channelId: id,
    login: name.toLowerCase(),
    displayName: name,
    title: 'speedrun',
    viewersLabel: '42 viewers',
    thumbnailUrl: 'https://cdn/$id.jpg',
  ),
  avatarUrl: 'https://cdn/$id-avatar.png',
  uptimeLabel: '3h 12m',
);

FavoritesUpdateSnapshot snapshot({
  List<LiveFavorite> live = const [],
  List<FavoriteChannel> offline = const [],
  List<FavoriteChannel> unchecked = const [],
}) =>
    FavoritesUpdateSnapshot(live: live, offline: offline, unchecked: unchecked);
