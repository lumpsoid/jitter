import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:jitter/top/browse_holder.dart';
import 'package:jitter/top/stream_card_tile.dart';

/// Cards are at most this wide; the grid adds columns as it widens.
const double _maxCardWidth = 360;

/// Height of a card's text below its 16:9 thumbnail.
const double _cardTextHeight = 72;

/// Scrolled this close to the end, the grid asks for the next page.
const double _nextPageExtent = 800;

/// The most-watched live channels: a grid of stream cards, refreshed by
/// pulling down and paged by scrolling.
class TopPage extends StatelessWidget {
  const new({required this.browse, this.onOpenStream, super.key});

  final BrowseHolder browse;

  /// Opens the tapped stream's channel by login. Cards are not tappable
  /// without it.
  final ValueChanged<String>? onOpenStream;

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: browse,
      builder: (context, _) => switch (browse.state) {
        BrowseStateFailed(:final message) => _Failure(
          message: message,
          onRetry: browse.refresh,
        ),
        BrowseStateLoaded(:final streams, :final more) => _Grid(
          browse: browse,
          streams: streams,
          more: more,
          onOpenStream: onOpenStream,
        ),
        _ => const Center(child: CircularProgressIndicator()),
      },
    );
  }
}

class _Grid extends StatelessWidget {
  const new({
    required this.browse,
    required this.streams,
    required this.more,
    required this.onOpenStream,
  });

  final BrowseHolder browse;
  final List<StreamCard> streams;
  final BrowseMore more;
  final ValueChanged<String>? onOpenStream;

  bool _onScroll(ScrollNotification notification) {
    if (notification.metrics.extentAfter < _nextPageExtent) browse.next();
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
          onRefresh: () async => browse.refresh(),
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
                        onTap: open == null ? null : () => open(card.login),
                      );
                    },
                  ),
                ),
                SliverToBoxAdapter(
                  child: _Footer(more: more, onRetry: browse.next),
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

  final BrowseMore more;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 16),
      child: switch (more) {
        BrowseMoreLoading() => const Center(child: CircularProgressIndicator()),
        BrowseMoreFailed(:final message) => _Failure(
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
