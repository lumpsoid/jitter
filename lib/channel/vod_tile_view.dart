import 'package:flutter/material.dart';
import 'package:jitter/src/bindings/bindings.dart';

/// Width of a VOD's thumbnail in the list.
const double _thumbnailWidth = 160;

/// One VOD: its thumbnail with the duration over it and, when playback
/// would resume part way, a bar of how far; then its title.
class VodTileView extends StatelessWidget {
  const new({required this.vod, this.onTap, super.key});

  final VodTile vod;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return InkWell(
      onTap: onTap,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            SizedBox(
              width: _thumbnailWidth,
              child: AspectRatio(
                aspectRatio: 16 / 9,
                child: ClipRRect(
                  borderRadius: BorderRadius.circular(4),
                  child: Stack(
                    fit: StackFit.expand,
                    children: [
                      Image.network(
                        vod.thumbnailUrl,
                        fit: BoxFit.cover,
                        errorBuilder: (context, error, stack) => ColoredBox(
                          color: theme.colorScheme.surfaceContainerHighest,
                        ),
                      ),
                      Positioned(
                        right: 4,
                        bottom: 6,
                        child: _DurationBadge(label: vod.durationLabel),
                      ),
                      if (vod.progress case final progress?)
                        Positioned(
                          left: 0,
                          right: 0,
                          bottom: 0,
                          child: LinearProgressIndicator(
                            value: progress,
                            minHeight: 3,
                          ),
                        ),
                    ],
                  ),
                ),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Text(
                vod.title,
                maxLines: 3,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.titleSmall,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _DurationBadge extends StatelessWidget {
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
        padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
        child: Text(
          label,
          style: Theme.of(context).textTheme.labelSmall
              ?.copyWith(color: Colors.white),
        ),
      ),
    );
  }
}
