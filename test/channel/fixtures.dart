import 'package:jitter/src/bindings/bindings.dart';

VodTile vod(String id, {double? progress}) => VodTile(
  id: id,
  title: 'Title $id',
  durationLabel: '1:02:03',
  thumbnailUrl: 'https://cdn/$id-320x180.jpg',
  progress: progress,
);

VodsUpdate loadedVods(
  List<String> ids, {
  VodsMore more = const VodsMoreIdle(),
}) => VodsUpdateSnapshot(
  state: VodsStateLoaded(vods: [for (final id in ids) vod(id)], more: more),
);

/// `xqc`, with no description unless given one.
ChannelUpdate found({
  ChannelLive live = const ChannelLiveOffline(),
  String? description,
}) => ChannelUpdateFound(
  channelId: '12',
  login: 'xqc',
  displayName: 'xQc',
  avatarUrl: 'https://cdn/avatar.png',
  description: description,
  live: live,
);
