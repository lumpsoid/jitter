import 'package:flutter/material.dart';
import 'package:jitter/favorites/favorites_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';

/// A heart that keeps the channel [channelId] as a favorite or stops
/// keeping it. Filled while [favorites] lists the channel; Rust looks the
/// channel up by [login] to add it. Drop it on any card or page that shows
/// a channel.
class FavoriteStar extends StatelessWidget {
  const new({
    required this.favorites,
    required this.channelId,
    required this.login,
    super.key,
  });

  final FavoritesHolder favorites;
  final String channelId;
  final String login;

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: favorites,
      builder: (context, _) {
        final starred = favorites.isFavorite(channelId);
        return IconButton(
          isSelected: starred,
          icon: const Icon(Icons.favorite_outline),
          selectedIcon: const Icon(Icons.favorite),
          tooltip: starred ? 'Remove from favorites' : 'Add to favorites',
          onPressed: starred
              ? () => RemoveFavorite(channelId: channelId).sendSignalToRust()
              : () => AddFavorite(login: login).sendSignalToRust(),
        );
      },
    );
  }
}
