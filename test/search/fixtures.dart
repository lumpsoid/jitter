import 'package:jitter/src/bindings/bindings.dart';

import '../categories/fixtures.dart';

ChannelResult channelResult(String login, {String? liveLabel}) => ChannelResult(
  login: login,
  displayName: 'Name $login',
  avatarUrl: 'https://cdn/$login-300x300.png',
  followersLabel: '39.9K followers',
  liveLabel: liveLabel,
);

SearchUpdate searchedChannels(
  List<String> logins, {
  SearchMore more = const SearchMoreIdle(),
}) => SearchUpdateChannels(
  state: SearchChannelsStateLoaded(
    channels: [for (final login in logins) channelResult(login)],
    more: more,
  ),
);

SearchUpdate searchedCategories(
  List<String> ids, {
  SearchMore more = const SearchMoreIdle(),
}) => SearchUpdateCategories(
  state: SearchCategoriesStateLoaded(
    categories: [for (final id in ids) tile(id)],
    more: more,
  ),
);
