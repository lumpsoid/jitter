import 'package:flutter/services.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:material_ui/material_ui.dart';

/// Width of a VOD's thumbnail in the list.
const double _thumbnailWidth = 160;

/// One VOD: its thumbnail with the duration over it and, when playback
/// would resume part way, a bar of how far; then its title, game and date.
/// Given [onDownload], a long press, or the menu key (Shift+F10), offers to
/// download the VOD.
class VodTileView extends StatelessWidget {
  const new({required this.vod, this.onTap, this.onDownload, super.key});

  final VodTile vod;
  final VoidCallback? onTap;
  final VoidCallback? onDownload;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final download = onDownload;
    final tile = InkWell(
      onTap: onTap,
      onLongPress: download == null
          ? null
          : () => _showActions(context, download),
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
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    vod.title,
                    maxLines: 3,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.titleSmall,
                  ),
                  if (vod.gameName case final game?)
                    Text(
                      game,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.bodySmall,
                    ),
                  if (vod.dateLabel case final date?)
                    Text(
                      date,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
    if (download == null) return tile;
    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.contextMenu): () =>
            _showActions(context, download),
        const SingleActivator(LogicalKeyboardKey.f10, shift: true): () =>
            _showActions(context, download),
      },
      child: tile,
    );
  }

  void _showActions(BuildContext context, VoidCallback download) =>
      showModalBottomSheet<void>(
        context: context,
        builder: (sheetContext) => SafeArea(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              ListTile(
                leading: const Icon(Icons.download_outlined),
                title: const Text('Download'),
                onTap: () {
                  Navigator.pop(sheetContext);
                  download();
                },
              ),
            ],
          ),
        ),
      );
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
