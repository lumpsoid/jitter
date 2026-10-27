import 'dart:async';

import 'package:flutter/material.dart';
import 'package:jitter/favorites/favorite_star.dart';
import 'package:jitter/favorites/favorites_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:jitter/top/stream_card_tile.dart';
import 'package:jitter/top/stream_grid.dart';

/// Size of one channel in the offline and not-checked grids.
const double _channelTileWidth = 104;
const double _channelTileHeight = 112;

/// Favorite channels, live first as stream cards in a grid, then the
/// offline ones and the ones not checked yet as compact avatar grids. Pull
/// to refresh. Shows every favorites failure as a snackbar, including
/// those from a [FavoriteStar] on another page: the shell keeps this page
/// alive.
class FavoritesPage extends StatefulWidget {
  const new({
    required this.favorites,
    required this.onOpenChannel,
    required this.onOpenLive,
    super.key,
  });

  final FavoritesHolder favorites;

  /// Opens the channel page for this login (an offline channel, or a live
  /// card's avatar or name).
  final void Function(String login) onOpenChannel;

  /// Opens the live player for this login (a tapped live card).
  final void Function(String login) onOpenLive;

  @override
  State<FavoritesPage> createState() => _FavoritesPageState();
}

class _FavoritesPageState extends State<FavoritesPage> {
  late final StreamSubscription<String> _failures;

  @override
  void initState() {
    super.initState();
    _failures = widget.favorites.failures.listen(_showFailure);
  }

  void _showFailure(String message) {
    ScaffoldMessenger.of(context)
        .showSnackBar(SnackBar(content: Text(message)));
  }

  @override
  void dispose() {
    unawaited(_failures.cancel());
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: widget.favorites,
      builder: (context, _) {
        final favorites = widget.favorites.favorites;
        if (favorites == null) {
          return const Center(child: CircularProgressIndicator());
        }
        return RefreshIndicator(
          onRefresh: widget.favorites.refresh,
          child: LayoutBuilder(
            builder: (context, constraints) => CustomScrollView(
              physics: const AlwaysScrollableScrollPhysics(),
              slivers: widget.favorites.isEmpty
                  ? const [SliverToBoxAdapter(child: _NoFavorites())]
                  : [
                      if (favorites.live.isNotEmpty) ...[
                        _Header(label: 'Live now · ${favorites.live.length}'),
                        _liveGrid(favorites.live, constraints.maxWidth),
                      ],
                      if (favorites.offline.isNotEmpty) ...[
                        _Header(label: 'Offline · ${favorites.offline.length}'),
                        _channelGrid(favorites.offline, dimmed: true),
                      ],
                      if (favorites.unchecked.isNotEmpty) ...[
                        _Header(
                          label:
                              'Not checked yet · '
                              '${favorites.unchecked.length}',
                        ),
                        _channelGrid(favorites.unchecked, dimmed: false),
                      ],
                      const SliverToBoxAdapter(child: SizedBox(height: 16)),
                    ],
            ),
          ),
        );
      },
    );
  }

  Widget _liveGrid(List<LiveFavorite> live, double width) {
    return SliverPadding(
      padding: const EdgeInsets.symmetric(horizontal: 8),
      sliver: SliverGrid.builder(
        gridDelegate: streamGridDelegate(width - 16),
        itemCount: live.length,
        itemBuilder: (context, index) {
          final favorite = live[index];
          final login = favorite.stream.login;
          return StreamCardTile(
            card: favorite.stream,
            favorites: widget.favorites,
            uptimeLabel: favorite.uptimeLabel,
            avatarUrl: favorite.avatarUrl,
            onTap: () => widget.onOpenLive(login),
            onOpenChannel: () => widget.onOpenChannel(login),
          );
        },
      ),
    );
  }

  Widget _channelGrid(List<FavoriteChannel> channels, {required bool dimmed}) {
    return SliverPadding(
      padding: const EdgeInsets.symmetric(horizontal: 8),
      sliver: SliverGrid.builder(
        gridDelegate: const SliverGridDelegateWithMaxCrossAxisExtent(
          maxCrossAxisExtent: _channelTileWidth,
          mainAxisExtent: _channelTileHeight,
        ),
        itemCount: channels.length,
        itemBuilder: (context, index) => _ChannelTile(
          channel: channels[index],
          favorites: widget.favorites,
          dimmed: dimmed,
          onOpenChannel: widget.onOpenChannel,
        ),
      ),
    );
  }
}

class _NoFavorites extends StatelessWidget {
  const new();

  @override
  Widget build(BuildContext context) {
    return const Padding(
      padding: EdgeInsets.all(32),
      child: Text(
        'No favorites yet. Star a channel to keep it here.',
        textAlign: TextAlign.center,
      ),
    );
  }
}

class _Header extends StatelessWidget {
  const new({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    return SliverToBoxAdapter(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 16, 16, 8),
        child: Text(label, style: Theme.of(context).textTheme.titleMedium),
      ),
    );
  }
}

/// A channel that is not live: its avatar in greyscale (or its initial when
/// no avatar is stored) and its name, with the favorite star in the corner.
/// [dimmed] fades it out (known offline).
class _ChannelTile extends StatelessWidget {
  const new({
    required this.channel,
    required this.favorites,
    required this.dimmed,
    required this.onOpenChannel,
  });

  final FavoriteChannel channel;
  final FavoritesHolder favorites;
  final bool dimmed;
  final void Function(String login) onOpenChannel;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final name = channel.displayName;
    return Stack(
      children: [
        Positioned.fill(
          child: Opacity(
            opacity: dimmed ? 0.5 : 1,
            child: InkWell(
              borderRadius: BorderRadius.circular(8),
              onTap: () => onOpenChannel(channel.login),
              child: Padding(
                padding: const EdgeInsets.all(8),
                child: Column(
                  children: [
                    _avatar(theme, name),
                    const SizedBox(height: 6),
                    Text(
                      name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      textAlign: TextAlign.center,
                      style: theme.textTheme.bodySmall,
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
        Positioned(
          top: 0,
          right: 0,
          child: Transform.scale(
            scale: 0.75,
            child: FavoriteStar(
              favorites: favorites,
              channelId: channel.channelId,
              login: channel.login,
            ),
          ),
        ),
      ],
    );
  }

  /// The stored avatar in greyscale (the channel is not live), over its
  /// initial: the initial shows when there is no avatar or it fails to load.
  Widget _avatar(ThemeData theme, String name) {
    final url = channel.avatarUrl;
    final avatar = CircleAvatar(
      radius: 28,
      backgroundColor: theme.colorScheme.surfaceContainerHighest,
      foregroundColor: theme.colorScheme.onSurfaceVariant,
      foregroundImage: url == null ? null : NetworkImage(url),
      // A failed avatar falls back to the initial.
      onForegroundImageError: url == null ? null : (_, _) {},
      child: Text(name.characters.take(1).toUpperCase().toString()),
    );
    return url == null
        ? avatar
        : ColorFiltered(colorFilter: _greyscale, child: avatar);
  }
}

/// Luminance-weighted greyscale (Rec. 709), alpha untouched.
const ColorFilter _greyscale = ColorFilter.matrix(<double>[
  0.2126, 0.7152, 0.0722, 0, 0, //
  0.2126, 0.7152, 0.0722, 0, 0, //
  0.2126, 0.7152, 0.0722, 0, 0, //
  0, 0, 0, 1, 0, //
]);
