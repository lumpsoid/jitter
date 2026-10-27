import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:jitter/favorites/favorites_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:jitter/top/stream_card_tile.dart';
import 'package:jitter/widgets/paged_scroll_view.dart';

/// Cards are at most this wide; the grid adds columns as it widens.
const double _maxCardWidth = 360;

/// Height of a card's text below its 16:9 thumbnail.
const double _cardTextHeight = 72;

/// Columns of stream cards sharing [width]: as many as keep each card at
/// most [_maxCardWidth] wide, each as tall as its 16:9 thumbnail plus text.
SliverGridDelegate streamGridDelegate(double width) {
  final columns = math.max(1, (width / _maxCardWidth).ceil());
  final cardWidth = width / columns;
  return SliverGridDelegateWithFixedCrossAxisCount(
    crossAxisCount: columns,
    mainAxisExtent: cardWidth * 9 / 16 + _cardTextHeight,
  );
}

/// [streams] as a grid of stream cards with [footer] under it, refreshed by
/// pulling down and paged by scrolling.
class PagedStreamGrid extends StatelessWidget {
  const new({
    required this.streams,
    required this.favorites,
    required this.footer,
    required this.onRefresh,
    required this.onNextPage,
    required this.onOpenStream,
    this.onOpenChannel,
    super.key,
  });

  final List<StreamCard> streams;

  /// Read by each card's favorite star.
  final FavoritesHolder favorites;

  /// Under the grid: the next page's progress.
  final Widget footer;

  final VoidCallback onRefresh;
  final VoidCallback onNextPage;

  /// Called with the tapped stream's login. Cards are not tappable without
  /// it.
  final ValueChanged<String>? onOpenStream;

  /// Called with the login of the stream whose channel name was tapped.
  /// Without it, tapping the name plays the stream like the rest of the
  /// card.
  final ValueChanged<String>? onOpenChannel;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) => PagedScrollView(
        onRefresh: () async => onRefresh(),
        onNextPage: onNextPage,
        slivers: [
          SliverPadding(
            padding: const EdgeInsets.all(8),
            sliver: SliverGrid.builder(
              gridDelegate: streamGridDelegate(constraints.maxWidth),
              itemCount: streams.length,
              itemBuilder: (context, index) {
                final card = streams[index];
                final open = onOpenStream;
                final openChannel = onOpenChannel;
                return StreamCardTile(
                  card: card,
                  favorites: favorites,
                  onTap: open == null ? null : () => open(card.login),
                  onOpenChannel: openChannel == null
                      ? null
                      : () => openChannel(card.login),
                );
              },
            ),
          ),
          SliverToBoxAdapter(child: footer),
        ],
      ),
    );
  }
}
