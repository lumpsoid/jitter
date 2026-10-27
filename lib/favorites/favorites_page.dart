import 'dart:async';

import 'package:flutter/material.dart';
import 'package:jitter/favorites/favorite_star.dart';
import 'package:jitter/favorites/favorites_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';

/// Favorite channels with live status, pull to refresh. Shows every
/// favorites failure as a snackbar, including those from a [FavoriteStar]
/// on another page: the shell keeps this page alive.
class FavoritesPage extends StatefulWidget {
  const new({required this.favorites, required this.onOpenChannel, super.key});

  final FavoritesHolder favorites;

  /// Opens the channel with this login (a tapped row).
  final void Function(String login) onOpenChannel;

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
          child: favorites.isEmpty
              ? ListView(
                  children: const [
                    Padding(
                      padding: EdgeInsets.all(32),
                      child: Text(
                        'No favorites yet. Star a channel to keep it here.',
                        textAlign: TextAlign.center,
                      ),
                    ),
                  ],
                )
              : ListView.builder(
                  itemCount: favorites.length,
                  itemBuilder: (context, index) => _FavoriteTile(
                    row: favorites[index],
                    favorites: widget.favorites,
                    onOpenChannel: widget.onOpenChannel,
                  ),
                ),
        );
      },
    );
  }
}

class _FavoriteTile extends StatelessWidget {
  const new({
    required this.row,
    required this.favorites,
    required this.onOpenChannel,
  });

  final FavoriteRow row;
  final FavoritesHolder favorites;
  final void Function(String login) onOpenChannel;

  @override
  Widget build(BuildContext context) {
    final avatarUrl = row.avatarUrl;
    final status = row.status;
    return ListTile(
      leading: CircleAvatar(
        foregroundImage: avatarUrl == null ? null : NetworkImage(avatarUrl),
        child: const Icon(Icons.person),
      ),
      title: Text(row.displayName),
      subtitle: switch (status) {
        FavoriteStatusLive(:final title, :final viewersLabel) => Text(
          '$viewersLabel · $title',
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
        ),
        FavoriteStatusOffline() => const Text('Offline'),
        _ => null,
      },
      trailing: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (status is FavoriteStatusLive) const _LiveBadge(),
          FavoriteStar(
            favorites: favorites,
            channelId: row.channelId,
            login: row.login,
          ),
        ],
      ),
      onTap: () => onOpenChannel(row.login),
    );
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
