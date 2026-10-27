import 'package:flutter/material.dart';
import 'package:jitter/channel/channel_about.dart';
import 'package:jitter/channel/channel_holder.dart';
import 'package:jitter/channel/vod_tile_view.dart';
import 'package:jitter/favorites/favorite_star.dart';
import 'package:jitter/favorites/favorites_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';

/// Scrolled this close to the end, the list asks for the next page.
const double _nextPageExtent = 800;

/// The VOD tabs, in [VodKind]'s order, then the About tab.
const List<String> _tabLabels = ['Archive', 'Highlights', 'Uploads', 'About'];

/// One channel: its header (avatar, name, favorite star, live stream if
/// any) above its VODs in tabs and its About tab, refreshed by pulling down
/// and paged by scrolling. Shows what [channel] holds: [push] opens [login]
/// in it first.
class ChannelPage extends StatefulWidget {
  const new({
    required this.login,
    required this.channel,
    required this.favorites,
    this.onOpenVod,
    this.onOpenLive,
    super.key,
  });

  final String login;
  final ChannelHolder channel;

  /// Read by the header's favorite star.
  final FavoritesHolder favorites;

  /// Plays the tapped VOD by id. VODs are not tappable without it.
  final ValueChanged<String>? onOpenVod;

  /// Plays the channel's live stream by login. The live stream is not
  /// tappable without it.
  final ValueChanged<String>? onOpenLive;

  /// Opens [login] in [channel], then pushes its page. A new page starts
  /// on its first tab, as Rust starts on its first kind.
  static Future<void> push(
    BuildContext context, {
    required String login,
    required ChannelHolder channel,
    required FavoritesHolder favorites,
    ValueChanged<String>? onOpenVod,
    ValueChanged<String>? onOpenLive,
  }) {
    channel.open(login);
    return Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (context) => ChannelPage(
          login: login,
          channel: channel,
          favorites: favorites,
          onOpenVod: onOpenVod,
          onOpenLive: onOpenLive,
        ),
      ),
    );
  }

  @override
  State<ChannelPage> createState() => _ChannelPageState();
}

class _ChannelPageState extends State<ChannelPage>
    with SingleTickerProviderStateMixin {
  // Kept across header rebuilds, so a refresh stays on the selected tab
  // (Rust keeps its kind too).
  late final TabController _tabs;

  @override
  void initState() {
    super.initState();
    _tabs = TabController(length: _tabLabels.length, vsync: this)
      ..addListener(_onTab);
  }

  /// The VOD kind the selected tab lists; `null`: the About tab.
  VodKind? get _kind => VodKind.values.elementAtOrNull(_tabs.index);

  // Coming back from About to the kind already listed, Rust keeps its list.
  void _onTab() {
    if (_tabs.indexIsChanging) return;
    if (_kind case final kind?) widget.channel.selectKind(kind);
  }

  @override
  void dispose() {
    _tabs.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final channel = widget.channel;
    return ListenableBuilder(
      listenable: Listenable.merge([channel, _tabs]),
      builder: (context, _) {
        final header = channel.header;
        return Scaffold(
          appBar: AppBar(
            title: Text(switch (header) {
              ChannelUpdateFound(:final displayName) => displayName,
              _ => widget.login,
            }),
            actions: [
              if (header case ChannelUpdateFound(
                :final channelId,
                :final login,
              ))
                FavoriteStar(
                  favorites: widget.favorites,
                  channelId: channelId,
                  login: login,
                ),
            ],
          ),
          body: switch (header) {
            ChannelUpdateNotFound(:final login) => _Message(
              text: 'No channel is called $login.',
            ),
            ChannelUpdateFailed(:final message) => _Failure(
              message: message,
              onRetry: channel.refresh,
            ),
            _ => Column(
              children: [
                _Header(
                  login: widget.login,
                  header: header,
                  onOpenLive: widget.onOpenLive,
                ),
                TabBar(
                  controller: _tabs,
                  tabs: [for (final label in _tabLabels) Tab(text: label)],
                ),
                Expanded(
                  child: switch ((_kind, header)) {
                    (VodKind(), _) => _Vods(
                      channel: channel,
                      onOpenVod: widget.onOpenVod,
                    ),
                    (
                      null,
                      ChannelUpdateFound(
                        :final followersLabel,
                        :final description,
                      ),
                    ) =>
                      ChannelAbout(
                        followersLabel: followersLabel,
                        description: description,
                        onRefresh: channel.refresh,
                      ),
                    (null, _) => const Center(
                      child: CircularProgressIndicator(),
                    ),
                  },
                ),
              ],
            ),
          },
        );
      },
    );
  }
}

/// The channel's avatar and name, then whether it is live. While it is
/// looked up, the same tile with [login] and no avatar, so the tabs below
/// do not move when it arrives.
class _Header extends StatelessWidget {
  const new({
    required this.login,
    required this.header,
    required this.onOpenLive,
  });

  final String login;
  final ChannelUpdate? header;
  final ValueChanged<String>? onOpenLive;

  @override
  Widget build(BuildContext context) {
    final header = this.header;
    if (header is! ChannelUpdateFound) {
      return ListTile(
        leading: const CircleAvatar(child: Icon(Icons.person)),
        title: Text(login),
        subtitle: const Text('Looking up the channel…'),
      );
    }
    return ListTile(
      leading: CircleAvatar(
        foregroundImage: NetworkImage(header.avatarUrl),
        // An avatar that fails to load leaves the icon.
        onForegroundImageError: (error, stack) {},
        child: const Icon(Icons.person),
      ),
      title: Text(header.displayName),
      subtitle: _LiveLine(
        live: header.live,
        onTap: switch (onOpenLive) {
          final open? => () => open(header.login),
          null => null,
        },
      ),
    );
  }
}

/// Whether the channel is live; when it is, its stream, tappable.
class _LiveLine extends StatelessWidget {
  const new({required this.live, required this.onTap});

  final ChannelLive live;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    return switch (live) {
      ChannelLiveLive(:final title, :final viewersLabel, :final gameName) =>
        InkWell(
          onTap: onTap,
          child: Row(
            children: [
              const _LiveBadge(),
              const SizedBox(width: 6),
              Expanded(
                child: Text(
                  [viewersLabel, ?gameName, title].join(' · '),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
          ),
        ),
      ChannelLiveOffline() => const Text('Offline'),
      ChannelLiveFailed(:final message) => Text(message),
      _ => const Text('Checking whether live…'),
    };
  }
}

class _LiveBadge extends StatelessWidget {
  const new();

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: colors.error,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
        child: Text(
          'LIVE',
          style: Theme.of(context).textTheme.labelSmall
              ?.copyWith(color: colors.onError),
        ),
      ),
    );
  }
}

/// The VODs of the selected tab.
class _Vods extends StatelessWidget {
  const new({required this.channel, required this.onOpenVod});

  final ChannelHolder channel;
  final ValueChanged<String>? onOpenVod;

  bool _onScroll(ScrollNotification notification) {
    if (notification.metrics.extentAfter < _nextPageExtent) channel.next();
    return false;
  }

  @override
  Widget build(BuildContext context) {
    return switch (channel.vods) {
      VodsStateFailed(:final message) => _Failure(
        message: message,
        onRetry: channel.refresh,
      ),
      VodsStateLoaded(:final vods, :final more) => RefreshIndicator(
        onRefresh: () async => channel.refresh(),
        child: NotificationListener<ScrollNotification>(
          onNotification: _onScroll,
          child: vods.isEmpty
              ? ListView(
                  physics: const AlwaysScrollableScrollPhysics(),
                  children: const [_Message(text: 'No videos here.')],
                )
              : ListView.builder(
                  physics: const AlwaysScrollableScrollPhysics(),
                  itemCount: vods.length + 1,
                  itemBuilder: (context, index) {
                    if (index == vods.length) {
                      return _Footer(more: more, onRetry: channel.next);
                    }
                    final vod = vods[index];
                    final open = onOpenVod;
                    return VodTileView(
                      vod: vod,
                      onTap: open == null ? null : () => open(vod.id),
                    );
                  },
                ),
        ),
      ),
      _ => const Center(child: CircularProgressIndicator()),
    };
  }
}

/// Under the list: a spinner while the next page loads, its failure with a
/// Retry, otherwise nothing.
class _Footer extends StatelessWidget {
  const new({required this.more, required this.onRetry});

  final VodsMore more;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 16),
      child: switch (more) {
        VodsMoreLoading() => const Center(child: CircularProgressIndicator()),
        VodsMoreFailed(:final message) => _Failure(
          message: message,
          onRetry: onRetry,
        ),
        _ => const SizedBox.shrink(),
      },
    );
  }
}

class _Message extends StatelessWidget {
  const new({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(32),
      child: Text(text, textAlign: TextAlign.center),
    );
  }
}

class _Failure extends StatelessWidget {
  const new({required this.message, required this.onRetry});

  final String message;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(message, textAlign: TextAlign.center),
            const SizedBox(height: 8),
            FilledButton.tonal(onPressed: onRetry, child: const Text('Retry')),
          ],
        ),
      ),
    );
  }
}
