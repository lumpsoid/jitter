import 'package:flutter/material.dart';
import 'package:jitter/categories/category_holder.dart';
import 'package:jitter/favorites/favorites_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:jitter/top/stream_grid.dart';
import 'package:jitter/widgets/escape_goes_back.dart';
import 'package:jitter/widgets/next_page_footer.dart';
import 'package:jitter/widgets/retry_failure.dart';

/// One category's live streams: the favorites playing it first, then the
/// rest by viewers, refreshed by pulling down and paged by scrolling.
/// Shows what [category] holds: [push] opens the category in it first.
class CategoryPage extends StatelessWidget {
  const new({
    required this.name,
    required this.category,
    required this.favorites,
    this.onOpenStream,
    this.onOpenChannel,
    super.key,
  });

  /// The category's name, for the title.
  final String name;
  final CategoryHolder category;

  /// Read by each card's favorite star.
  final FavoritesHolder favorites;

  /// Plays the tapped stream by login. Cards are not tappable without it.
  final ValueChanged<String>? onOpenStream;

  /// Opens the channel of the stream whose channel name was tapped, by
  /// login.
  final ValueChanged<String>? onOpenChannel;

  /// Opens category [id] in [category], then pushes its page.
  static Future<void> push(
    BuildContext context, {
    required String id,
    required String name,
    required CategoryHolder category,
    required FavoritesHolder favorites,
    ValueChanged<String>? onOpenStream,
    ValueChanged<String>? onOpenChannel,
  }) {
    category.open(id);
    return Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (context) => EscapeGoesBack(
          child: CategoryPage(
            name: name,
            category: category,
            favorites: favorites,
            onOpenStream: onOpenStream,
            onOpenChannel: onOpenChannel,
          ),
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
          CategoryStateFailed(:final message) => RetryFailure(
            message: message,
            onRetry: category.refresh,
          ),
          CategoryStateLoaded(
            favorites: final pinned,
            :final streams,
            :final more,
          ) =>
            PagedStreamGrid(
              streams: [...pinned, ...streams],
              favorites: favorites,
              footer: switch (more) {
                CategoryMoreLoading() => const NextPageFooter.loading(),
                CategoryMoreFailed(:final message) => NextPageFooter.failed(
                  message: message,
                  onRetry: category.next,
                ),
                _ => const NextPageFooter.idle(),
              },
              onRefresh: category.refresh,
              onNextPage: category.next,
              onOpenStream: onOpenStream,
              onOpenChannel: onOpenChannel,
            ),
          _ => const Center(child: CircularProgressIndicator()),
        },
      ),
    );
  }
}
