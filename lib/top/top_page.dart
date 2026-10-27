import 'package:flutter/material.dart';
import 'package:jitter/favorites/favorites_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:jitter/top/browse_holder.dart';
import 'package:jitter/top/stream_grid.dart';
import 'package:jitter/widgets/next_page_footer.dart';
import 'package:jitter/widgets/retry_failure.dart';

/// The most-watched live channels: a grid of stream cards, refreshed by
/// pulling down and paged by scrolling.
class TopPage extends StatelessWidget {
  const new({
    required this.browse,
    required this.favorites,
    this.onOpenStream,
    super.key,
  });

  final BrowseHolder browse;

  /// Read by each card's favorite star.
  final FavoritesHolder favorites;

  /// Opens the tapped stream's channel by login. Cards are not tappable
  /// without it.
  final ValueChanged<String>? onOpenStream;

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: browse,
      builder: (context, _) => switch (browse.state) {
        BrowseStateFailed(:final message) => RetryFailure(
          message: message,
          onRetry: browse.refresh,
        ),
        BrowseStateLoaded(:final streams, :final more) => PagedStreamGrid(
          streams: streams,
          favorites: favorites,
          footer: switch (more) {
            BrowseMoreLoading() => const NextPageFooter.loading(),
            BrowseMoreFailed(:final message) => NextPageFooter.failed(
              message: message,
              onRetry: browse.next,
            ),
            _ => const NextPageFooter.idle(),
          },
          onRefresh: browse.refresh,
          onNextPage: browse.next,
          onOpenStream: onOpenStream,
        ),
        _ => const Center(child: CircularProgressIndicator()),
      },
    );
  }
}
