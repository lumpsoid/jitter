import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:jitter/categories/category_holder.dart';
import 'package:jitter/favorites/favorites_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:jitter/top/stream_card_tile.dart';

/// Cards are at most this wide; the grid adds columns as it widens.
const double _maxCardWidth = 360;

/// Height of a card's text below its 16:9 thumbnail.
const double _cardTextHeight = 72;

/// Scrolled this close to the end, the grid asks for the next page.
const double _nextPageExtent = 800;

/// One category's live streams: the favorites playing it first, then the
/// rest by viewers, refreshed by pulling down and paged by scrolling.
/// Shows what [category] holds: [push] opens the category in it first.
class CategoryPage extends StatelessWidget {
  const new({
    required this.name,
    required this.category,
    required this.favorites,
    this.onOpenStream,
    super.key,
  });

  /// The category's name, for the title.
  final String name;
  final CategoryHolder category;

  /// Read by each card's favorite star.
  final FavoritesHolder favorites;

  /// Plays the tapped stream by login. Cards are not tappable without it.
  final ValueChanged<String>? onOpenStream;

  /// Opens category [id] in [category], then pushes its page.
  static Future<void> push(
    BuildContext context, {
    required String id,
    required String name,
    required CategoryHolder category,
    required FavoritesHolder favorites,
    ValueChanged<String>? onOpenStream,
  }) {
    category.open(id);
    return Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (context) => CategoryPage(
          name: name,
          category: category,
          favorites: favorites,
          onOpenStream: onOpenStream,
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: Text(name)),
      body: ListenableBuilder(
        listenable: category,
        builder: (context, _) => switch (category.state) {
          CategoryStateFailed(:final message) => _Failure(
            message: message,
            onRetry: category.refresh,
          ),
          CategoryStateLoaded(
            favorites: final pinned,
            :final streams,
            :final more,
          ) =>
            _Grid(
              category: category,
              favorites: favorites,
              streams: [...pinned, ...streams],
              more: more,
              onOpenStream: onOpenStream,
            ),
          _ => const Center(child: CircularProgressIndicator()),
        },
      ),
    );
  }
}

class _Grid extends StatelessWidget {
  const new({
    required this.category,
    required this.favorites,
    required this.streams,
    required this.more,
    required this.onOpenStream,
  });

  final CategoryHolder category;
  final FavoritesHolder favorites;

  /// The pinned favorites, then the rest.
  final List<StreamCard> streams;
  final CategoryMore more;
  final ValueChanged<String>? onOpenStream;

  bool _onScroll(ScrollNotification notification) {
    if (notification.metrics.extentAfter < _nextPageExtent) category.next();
    return false;
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final columns = math.max(
          1,
          (constraints.maxWidth / _maxCardWidth).ceil(),
        );
        final cardWidth = constraints.maxWidth / columns;
        return RefreshIndicator(
          onRefresh: () async => category.refresh(),
          child: NotificationListener<ScrollNotification>(
            onNotification: _onScroll,
            child: CustomScrollView(
              physics: const AlwaysScrollableScrollPhysics(),
              slivers: [
                SliverPadding(
                  padding: const EdgeInsets.all(8),
                  sliver: SliverGrid.builder(
                    gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
                      crossAxisCount: columns,
                      mainAxisExtent: cardWidth * 9 / 16 + _cardTextHeight,
                    ),
                    itemCount: streams.length,
                    itemBuilder: (context, index) {
                      final card = streams[index];
                      final open = onOpenStream;
                      return StreamCardTile(
                        card: card,
                        favorites: favorites,
                        onTap: open == null ? null : () => open(card.login),
                      );
                    },
                  ),
                ),
                SliverToBoxAdapter(
                  child: _Footer(more: more, onRetry: category.next),
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}

/// Under the grid: a spinner while the next page loads, its failure with a
/// Retry, otherwise nothing.
class _Footer extends StatelessWidget {
  const new({required this.more, required this.onRetry});

  final CategoryMore more;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 16),
      child: switch (more) {
        CategoryMoreLoading() => const Center(
          child: CircularProgressIndicator(),
        ),
        CategoryMoreFailed(:final message) => _Failure(
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
