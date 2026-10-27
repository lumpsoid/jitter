import 'package:flutter/material.dart';
import 'package:jitter/src/bindings/bindings.dart';

/// Width over height of a category's box art.
const double boxArtAspectRatio = 3 / 4;

/// Height of a tile's text below its box art.
const double categoryTileTextHeight = 48;

/// One category: its portrait box art, then its name and viewers.
class CategoryTileView extends StatelessWidget {
  const new({required this.tile, this.onTap, super.key});

  final CategoryTile tile;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Card(
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            AspectRatio(
              aspectRatio: boxArtAspectRatio,
              child: Image.network(
                tile.boxArtUrl,
                fit: BoxFit.cover,
                errorBuilder: (context, error, stack) => ColoredBox(
                  color: theme.colorScheme.surfaceContainerHighest,
                ),
              ),
            ),
            Expanded(
              child: Padding(
                padding: const EdgeInsets.fromLTRB(8, 4, 8, 4),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      tile.name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.titleSmall,
                    ),
                    Text(
                      tile.viewersLabel,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
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
}
