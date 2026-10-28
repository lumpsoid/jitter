import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:jitter/categories/categories_page.dart';
import 'package:jitter/categories/category_tile_view.dart';
import 'package:jitter/search/search_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:jitter/widgets/next_page_footer.dart';
import 'package:jitter/widgets/paged_scroll_view.dart';
import 'package:jitter/widgets/retry_failure.dart';
import 'package:jitter/widgets/steam_keyboard.dart';

/// Category tiles are at most this wide; the grid adds columns as it
/// widens.
const double _maxTileWidth = 180;

/// A search field over the channels and the categories matching it, in
/// two tabs. Each list is refreshed by pulling down and paged by
/// scrolling.
class SearchPage extends StatefulWidget {
  const new({
    required this.search,
    this.onOpenChannel,
    this.onOpenCategory,
    super.key,
  });

  final SearchHolder search;

  /// Opens the tapped channel's page by login. Channels are not tappable
  /// without it.
  final ValueChanged<String>? onOpenChannel;

  /// Opens the tapped category. Categories are not tappable without it.
  final CategoryOpener? onOpenCategory;

  @override
  State<SearchPage> createState() => _SearchPageState();
}

class _SearchPageState extends State<SearchPage> {
  final TextEditingController _field = TextEditingController();
  final FocusNode _fieldFocus = FocusNode();

  @override
  void dispose() {
    _field.dispose();
    _fieldFocus.dispose();
    super.dispose();
  }

  void _clear() {
    _field.clear();
    widget.search.type('');
  }

  /// Steam's keyboard button, where there is one, and Clear once typed.
  Widget? _suffix({required bool typed}) {
    final clear = typed
        ? IconButton(
            tooltip: 'Clear',
            icon: const Icon(Icons.clear),
            onPressed: _clear,
          )
        : null;
    if (!SteamKeyboardButton.shown) return clear;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        SteamKeyboardButton(field: _fieldFocus),
        ?clear,
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final search = widget.search;
    return DefaultTabController(
      length: SearchList.values.length,
      child: Column(
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 12, 16, 4),
            child: ValueListenableBuilder(
              valueListenable: _field,
              // Down leaves the field for the results, as from any other
              // widget: the field would keep it for its caret.
              builder: (context, value, _) => Shortcuts(
                shortcuts: const {
                  SingleActivator(
                    LogicalKeyboardKey.arrowDown,
                  ): DirectionalFocusIntent(
                    TraversalDirection.down,
                    ignoreTextFields: false,
                  ),
                },
                child: TextField(
                  controller: _field,
                  focusNode: _fieldFocus,
                  onChanged: search.type,
                  textInputAction: TextInputAction.search,
                  decoration: InputDecoration(
                    hintText: 'Search channels and categories',
                    prefixIcon: const Icon(Icons.search),
                    suffixIcon: _suffix(typed: value.text.isNotEmpty),
                    border: const OutlineInputBorder(),
                  ),
                ),
              ),
            ),
          ),
          const TabBar(
            tabs: [
              Tab(text: 'Channels'),
              Tab(text: 'Categories'),
            ],
          ),
          Expanded(
            child: ListenableBuilder(
              listenable: search,
              builder: (context, _) => TabBarView(
                children: [
                  _Channels(
                    search: search,
                    state: search.channels,
                    onOpenChannel: widget.onOpenChannel,
                  ),
                  _Categories(
                    search: search,
                    state: search.categories,
                    onOpenCategory: widget.onOpenCategory,
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _Channels extends StatelessWidget {
  const new({
    required this.search,
    required this.state,
    required this.onOpenChannel,
  });

  final SearchHolder search;
  final SearchChannelsState state;
  final ValueChanged<String>? onOpenChannel;

  @override
  Widget build(BuildContext context) {
    const list = SearchList.channels;
    return switch (state) {
      SearchChannelsStateLoading() => const Center(
        child: CircularProgressIndicator(),
      ),
      SearchChannelsStateFailed(:final message) => RetryFailure(
        message: message,
        onRetry: () => search.refresh(list),
      ),
      SearchChannelsStateLoaded(:final channels) when channels.isEmpty =>
        const _Message('No channels match.'),
      SearchChannelsStateLoaded(:final channels, :final more) =>
        PagedScrollView(
          onRefresh: () async => search.refresh(list),
          onNextPage: () => search.next(list),
          slivers: [
            SliverList.builder(
              itemCount: channels.length,
              itemBuilder: (context, index) {
                final channel = channels[index];
                final open = onOpenChannel;
                return _ChannelRow(
                  channel: channel,
                  onTap: open == null ? null : () => open(channel.login),
                );
              },
            ),
            SliverToBoxAdapter(child: _footer(more, () => search.next(list))),
          ],
        ),
      _ => const _Message('Find channels and categories by name.'),
    };
  }
}

class _Categories extends StatelessWidget {
  const new({
    required this.search,
    required this.state,
    required this.onOpenCategory,
  });

  final SearchHolder search;
  final SearchCategoriesState state;
  final CategoryOpener? onOpenCategory;

  @override
  Widget build(BuildContext context) {
    const list = SearchList.categories;
    return switch (state) {
      SearchCategoriesStateLoading() => const Center(
        child: CircularProgressIndicator(),
      ),
      SearchCategoriesStateFailed(:final message) => RetryFailure(
        message: message,
        onRetry: () => search.refresh(list),
      ),
      SearchCategoriesStateLoaded(:final categories) when categories.isEmpty =>
        const _Message('No categories match.'),
      SearchCategoriesStateLoaded(:final categories, :final more) =>
        LayoutBuilder(
          builder: (context, constraints) {
            final columns = math.max(
              2,
              (constraints.maxWidth / _maxTileWidth).ceil(),
            );
            final tileWidth = constraints.maxWidth / columns;
            final open = onOpenCategory;
            return PagedScrollView(
              onRefresh: () async => search.refresh(list),
              onNextPage: () => search.next(list),
              slivers: [
                SliverPadding(
                  padding: const EdgeInsets.all(8),
                  sliver: SliverGrid.builder(
                    gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
                      crossAxisCount: columns,
                      mainAxisExtent:
                          tileWidth / boxArtAspectRatio +
                          categoryTileTextHeight,
                    ),
                    itemCount: categories.length,
                    itemBuilder: (context, index) {
                      final tile = categories[index];
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
                  child: _footer(more, () => search.next(list)),
                ),
              ],
            );
          },
        ),
      _ => const _Message('Find channels and categories by name.'),
    };
  }
}

/// The footer under a loaded list: [more]'s spinner or failure.
Widget _footer(SearchMore more, VoidCallback onRetry) => switch (more) {
  SearchMoreLoading() => const NextPageFooter.loading(),
  SearchMoreFailed(:final message) => NextPageFooter.failed(
    message: message,
    onRetry: onRetry,
  ),
  _ => const NextPageFooter.idle(),
};

/// One channel: its avatar, its name with its followers, and its live
/// viewers while it is live.
class _ChannelRow extends StatelessWidget {
  const new({required this.channel, required this.onTap});

  final ChannelResult channel;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final avatar = channel.avatarUrl;
    final followers = channel.followersLabel;
    final live = channel.liveLabel;
    return ListTile(
      onTap: onTap,
      leading: CircleAvatar(
        foregroundImage: avatar == null ? null : NetworkImage(avatar),
        // A failed avatar leaves the initial showing.
        onForegroundImageError: avatar == null ? null : (_, _) {},
        child: Text(channel.displayName.characters.take(1).toString()),
      ),
      title: Text(
        channel.displayName,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      subtitle: followers == null ? null : Text(followers),
      trailing: live == null
          ? null
          : Text(
              '● $live',
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.error,
              ),
            ),
    );
  }
}

class _Message extends StatelessWidget {
  const new(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Text(
          text,
          textAlign: TextAlign.center,
          style: theme.textTheme.bodyMedium?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
      ),
    );
  }
}
