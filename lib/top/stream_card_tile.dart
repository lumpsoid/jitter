import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:jitter/favorites/favorite_star.dart';
import 'package:jitter/favorites/favorites_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';

/// One live stream: its thumbnail with the viewer count (and the uptime,
/// when given) over it, then the title, the channel and the game, with the
/// channel's favorite star. Given an [avatarUrl], the channel's avatar
/// leads the text; given [onOpenChannel], tapping the avatar or the
/// channel name opens the channel instead of [onTap]. A long press offers
/// to open the channel (given [onOpenChannel]), or to copy the title, the
/// channel name or the game.
class StreamCardTile extends StatelessWidget {
  const new({
    required this.card,
    required this.favorites,
    this.onTap,
    this.uptimeLabel,
    this.avatarUrl,
    this.onOpenChannel,
    super.key,
  });

  final StreamCard card;
  final FavoritesHolder favorites;
  final VoidCallback? onTap;
  final String? uptimeLabel;
  final String? avatarUrl;
  final VoidCallback? onOpenChannel;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final muted = theme.textTheme.bodySmall?.copyWith(
      color: theme.colorScheme.onSurfaceVariant,
    );
    return Card(
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        onLongPress: () => _showActions(context),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            AspectRatio(
              aspectRatio: 16 / 9,
              child: Stack(
                fit: StackFit.expand,
                children: [
                  Image.network(
                    card.thumbnailUrl,
                    fit: BoxFit.cover,
                    errorBuilder: (context, error, stack) => ColoredBox(
                      color: theme.colorScheme.surfaceContainerHighest,
                    ),
                  ),
                  Positioned(
                    left: 6,
                    bottom: 6,
                    child: _ThumbnailBadge(label: card.viewersLabel),
                  ),
                  if (uptimeLabel case final uptime?)
                    Positioned(
                      right: 6,
                      bottom: 6,
                      child: _ThumbnailBadge(label: uptime),
                    ),
                ],
              ),
            ),
            Expanded(
              child: Padding(
                padding: const EdgeInsets.fromLTRB(8, 6, 0, 4),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    if (avatarUrl case final avatar?)
                      Padding(
                        padding: const EdgeInsets.only(right: 8, top: 2),
                        child: InkWell(
                          onTap: onOpenChannel,
                          customBorder: const CircleBorder(),
                          child: CircleAvatar(
                            radius: 16,
                            foregroundImage: NetworkImage(avatar),
                            // A failed avatar falls back to the icon.
                            onForegroundImageError: (_, _) {},
                            child: const Icon(Icons.person, size: 18),
                          ),
                        ),
                      ),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            card.title,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: theme.textTheme.titleSmall,
                          ),
                          InkWell(
                            onTap: onOpenChannel,
                            child: Text(
                              card.displayName,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: muted,
                            ),
                          ),
                          if (card.gameName case final game?)
                            Text(
                              game,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: muted,
                            ),
                        ],
                      ),
                    ),
                    FavoriteStar(
                      favorites: favorites,
                      channelId: card.channelId,
                      login: card.login,
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  void _showActions(BuildContext context) => unawaited(
    showModalBottomSheet<void>(
      context: context,
      builder: (sheetContext) => SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (onOpenChannel case final open?)
              ListTile(
                leading: const Icon(Icons.person),
                title: const Text('Open channel'),
                subtitle: Text(
                  card.displayName,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                onTap: () {
                  Navigator.pop(sheetContext);
                  open();
                },
              ),
            _CopyTile(label: 'Copy stream title', text: card.title),
            _CopyTile(label: 'Copy channel name', text: card.displayName),
            if (card.gameName case final game?)
              _CopyTile(label: 'Copy game name', text: game),
          ],
        ),
      ),
    ),
  );
}

/// A sheet row that puts [text] on the clipboard and closes the sheet.
class _CopyTile extends StatelessWidget {
  const new({required this.label, required this.text});

  final String label;
  final String text;

  @override
  Widget build(BuildContext context) {
    return ListTile(
      leading: const Icon(Icons.copy),
      title: Text(label),
      subtitle: Text(text, maxLines: 1, overflow: TextOverflow.ellipsis),
      onTap: () async {
        final messenger = ScaffoldMessenger.maybeOf(context);
        Navigator.pop(context);
        await Clipboard.setData(ClipboardData(text: text));
        messenger?.showSnackBar(SnackBar(content: Text('Copied "$text"')));
      },
    );
  }
}

class _ThumbnailBadge extends StatelessWidget {
  const new({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: Colors.black54,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
        child: Text(
          label,
          style: Theme.of(context).textTheme.labelSmall
              ?.copyWith(color: Colors.white),
        ),
      ),
    );
  }
}
