import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:jitter/categories/categories_holder.dart';
import 'package:jitter/categories/category_tile_view.dart';
import 'package:jitter/src/bindings/bindings.dart';

/// Tiles are at most this wide; the grid adds columns as it widens.
const double _maxTileWidth = 180;

/// Height of the "From your channels" strip.
const double _stripHeight = 72;

/// Scrolled this close to the end, the grid asks for the next page.
const double _nextPageExtent = 800;

/// Opens a category by id; its name titles the page until Rust answers.
typedef CategoryOpener = void Function(String id, String name);

/// The categories directory: what your live favorites are streaming, then
/// the most-watched categories as a grid of box art, refreshed by pulling
/// down and paged by scrolling.
class CategoriesPage extends StatelessWidget {
  const new({required this.categories, this.onOpenCategory, super.key});

  final CategoriesHolder categories;

  /// Opens the tapped category. Categories are not tappable without it.
  final CategoryOpener? onOpenCategory;

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: categories,
      builder: (context, _) => switch (categories.state) {
        CategoriesStateFailed(:final message) => _Failure(
          message: message,
          onRetry: categories.refresh,
        ),
        CategoriesStateLoaded(categories: final tiles, :final more) => _Grid(
          categories: categories,
          tiles: tiles,
          more: more,
          onOpenCategory: onOpenCategory,
        ),
        _ => const Center(child: CircularProgressIndicator()),
      },
    );
  }
}

class _Grid extends StatelessWidget {
  const new({
    required this.categories,
    required this.tiles,
    required this.more,
    required this.onOpenCategory,
  });

  final CategoriesHolder categories;
  final List<CategoryTile> tiles;
  final CategoriesMore more;
  final CategoryOpener? onOpenCategory;

  bool _onScroll(ScrollNotification notification) {
    if (notification.metrics.extentAfter < _nextPageExtent) categories.next();
    return false;
  }

  @override
  Widget build(BuildContext context) {
    final strip = categories.fromYourChannels;
    final open = onOpenCategory;
    return LayoutBuilder(
      builder: (context, constraints) {
        final columns = math.max(
          2,
          (constraints.maxWidth / _maxTileWidth).ceil(),
        );
        final tileWidth = constraints.maxWidth / columns;
        return RefreshIndicator(
          onRefresh: () async => categories.refresh(),
          child: NotificationListener<ScrollNotification>(
            onNotification: _onScroll,
            child: CustomScrollView(
              physics: const AlwaysScrollableScrollPhysics(),
              slivers: [
                if (strip.isNotEmpty) ...[
                  const _Heading('From your channels'),
                  SliverToBoxAdapter(
                    child: _Strip(rows: strip, onOpenCategory: open),
                  ),
                  const _Heading('Top categories'),
                ],
                SliverPadding(
                  padding: const EdgeInsets.all(8),
                  sliver: SliverGrid.builder(
                    gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
                      crossAxisCount: columns,
                      mainAxisExtent:
                          tileWidth / boxArtAspectRatio +
                          categoryTileTextHeight,
                    ),
                    itemCount: tiles.length,
                    itemBuilder: (context, index) {
                      final tile = tiles[index];
                      return CategoryTileView(
                        tile: tile,
                        onTap: open == null
                            ? null
                            : () => open(tile.id, tile.name),
                      );
                    },
                  ),
                ),
                SliverToBoxAdapter(
                  child: _Footer(more: more, onRetry: categories.next),
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}

class _Heading extends StatelessWidget {
  const new(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    return SliverPadding(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
      sliver: SliverToBoxAdapter(
        child: Text(text, style: Theme.of(context).textTheme.titleMedium),
      ),
    );
  }
}

/// The categories your live favorites are streaming, side by side: each
/// with the favorites playing it.
class _Strip extends StatelessWidget {
  const new({required this.rows, required this.onOpenCategory});

  final List<FavoritesCategoryRow> rows;
  final CategoryOpener? onOpenCategory;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final open = onOpenCategory;
    return SizedBox(
      height: _stripHeight,
      child: ListView.builder(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        itemCount: rows.length,
        itemBuilder: (context, index) {
          final row = rows[index];
          return Card(
            clipBehavior: Clip.antiAlias,
            child: InkWell(
              onTap: open == null ? null : () => open(row.id, row.name),
              child: ConstrainedBox(
                constraints: const BoxConstraints(maxWidth: 260),
                child: Padding(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 12,
                    vertical: 8,
                  ),
                  child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        row.name,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: theme.textTheme.titleSmall,
                      ),
                      Text(
                        '▸ ${row.channels.join(', ')}',
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
            ),
          );
        },
      ),
    );
  }
}

/// Under the grid: a spinner while the next page loads, its failure with a
/// Retry, otherwise nothing.
class _Footer extends StatelessWidget {
  const new({required this.more, required this.onRetry});

  final CategoriesMore more;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 16),
      child: switch (more) {
        CategoriesMoreLoading() => const Center(
          child: CircularProgressIndicator(),
        ),
        CategoriesMoreFailed(:final message) => _Failure(
          message: message,
          onRetry: onRetry,
        ),
        _ => const SizedBox.shrink(),
      },
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
