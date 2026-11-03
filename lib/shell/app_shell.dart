import 'dart:async';

import 'package:flutter/material.dart';
import 'package:jitter/categories/categories_holder.dart';
import 'package:jitter/categories/categories_page.dart';
import 'package:jitter/categories/category_holder.dart';
import 'package:jitter/categories/category_page.dart';
import 'package:jitter/channel/channel_holder.dart';
import 'package:jitter/channel/channel_page.dart';
import 'package:jitter/chat/chat_holder.dart';
import 'package:jitter/favorites/favorites_holder.dart';
import 'package:jitter/favorites/favorites_page.dart';
import 'package:jitter/history/history_holder.dart';
import 'package:jitter/history/history_page.dart';
import 'package:jitter/logs/logs_holder.dart';
import 'package:jitter/player/media_kit_engine.dart';
import 'package:jitter/player/playback_holder.dart';
import 'package:jitter/player/player_page.dart';
import 'package:jitter/search/search_holder.dart';
import 'package:jitter/search/search_page.dart';
import 'package:jitter/settings/settings_holder.dart';
import 'package:jitter/settings/settings_page.dart';
import 'package:jitter/shell/quit_button.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:jitter/top/browse_holder.dart';
import 'package:jitter/top/top_page.dart';
import 'package:jitter/widgets/escape_goes_back.dart';
import 'package:jitter/widgets/refresh_key.dart';

/// Below this width the destinations sit in a bottom bar, from it in a
/// side rail (Material's compact window class).
const double _railMinWidth = 600;

/// One top-level page in the navigation.
typedef _Destination = ({
  String label,
  IconData icon,
  IconData selectedIcon,
  Widget page,
  // What Ctrl+R refreshes on the page; `null`: nothing.
  VoidCallback? refresh,
});

/// The booted app: top-level pages behind a bottom bar (narrow) or a side
/// rail (wide). A feature with its own top-level page adds a destination
/// here; its holder comes in through the constructor.
class AppShell extends StatefulWidget {
  const new({
    // Browse
    required this.browse,
    // Categories
    required this.categories,
    required this.category,
    // Favorites
    required this.favorites,
    // History
    required this.history,
    // Playback
    required this.playback,
    required this.player,
    // Channel
    required this.channel,
    // Chat
    required this.chat,
    // Search
    required this.search,
    // Settings
    required this.settings,
    required this.logs,
    super.key,
  });

  // Browse
  final BrowseHolder browse;
  // Categories
  final CategoriesHolder categories;
  final CategoryHolder category;
  // Favorites
  final FavoritesHolder favorites;
  // History
  final HistoryHolder history;
  // Playback: what a page needs to push a `PlayerPage`.
  final PlaybackHolder playback;
  final MediaKitEngine player;
  // Channel
  final ChannelHolder channel;
  // Chat: shown on the player page.
  final ChatHolder chat;
  // Search
  final SearchHolder search;
  // Settings
  final SettingsHolder settings;
  final LogsHolder logs;

  @override
  State<AppShell> createState() => _AppShellState();
}

class _AppShellState extends State<AppShell> {
  int _selected = 0;

  List<_Destination> get _destinations => [
    (
      label: 'Top',
      icon: Icons.trending_up_outlined,
      selectedIcon: Icons.trending_up,
      page: TopPage(
        browse: widget.browse,
        favorites: widget.favorites,
        onOpenStream: _openLive,
        onOpenChannel: _openChannel,
      ),
      refresh: widget.browse.refresh,
    ),
    (
      label: 'Categories',
      icon: Icons.category_outlined,
      selectedIcon: Icons.category,
      page: CategoriesPage(
        categories: widget.categories,
        onOpenCategory: _openCategory,
      ),
      refresh: widget.categories.refresh,
    ),
    (
      label: 'Favorites',
      icon: Icons.favorite_outline,
      selectedIcon: Icons.favorite,
      page: FavoritesPage(
        favorites: widget.favorites,
        onOpenChannel: _openChannel,
        onOpenLive: _openLive,
      ),
      refresh: () => unawaited(widget.favorites.refresh()),
    ),
    (
      label: 'History',
      icon: Icons.history_outlined,
      selectedIcon: Icons.history,
      page: HistoryPage(
        history: widget.history,
        onOpenLive: _openLive,
        onOpenVod: _openVod,
        onOpenChannel: _openChannel,
      ),
      refresh: null,
    ),
    (
      label: 'Search',
      icon: Icons.search_outlined,
      selectedIcon: Icons.search,
      page: SearchPage(
        search: widget.search,
        onOpenChannel: _openChannel,
        onOpenCategory: _openCategory,
      ),
      refresh: null,
    ),
    (
      label: 'Settings',
      icon: Icons.settings_outlined,
      selectedIcon: Icons.settings,
      page: SettingsPage(settings: widget.settings, logs: widget.logs),
      refresh: null,
    ),
  ];

  // Categories
  void _openCategory(String id, String name) => unawaited(
    CategoryPage.push(
      context,
      id: id,
      name: name,
      category: widget.category,
      favorites: widget.favorites,
      onOpenStream: _openLive,
      onOpenChannel: _openChannel,
    ),
  );

  // Channel
  void _openChannel(String login) => unawaited(
    ChannelPage.push(
      context,
      login: login,
      channel: widget.channel,
      favorites: widget.favorites,
      onOpenVod: _openVod,
      onOpenLive: _openLive,
    ),
  );

  /// Opens the channel in place of the player page at [playerContext].
  void _replacePlayerWithChannel(BuildContext playerContext, String login) =>
      unawaited(
        ChannelPage.replace(
          playerContext,
          login: login,
          channel: widget.channel,
          favorites: widget.favorites,
          onOpenVod: _openVod,
          onOpenLive: _openLive,
        ),
      );

  void _openVod(String id) => _openPlayer(PlaybackTargetVod(id: id), id);

  void _openLive(String login) =>
      _openPlayer(PlaybackTargetLive(login: login), login);

  void _select(int index) => setState(() => _selected = index);

  /// Pushes the player for `target`; leaving the page stops it. Opening the
  /// channel from it replaces it, so player pages never stack (they share
  /// one player and one playback state).
  void _openPlayer(PlaybackTarget target, String title) {
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (playerContext) => EscapeGoesBack(
          child: PlayerPage(
            target: target,
            title: title,
            playback: widget.playback,
            player: widget.player,
            chat: widget.chat,
            onOpenChannel: (login) =>
                _replacePlayerWithChannel(playerContext, login),
          ),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final destinations = _destinations;
    // Keeps each page alive, so switching back keeps its scroll position;
    // Ctrl+R refreshes the one shown.
    final pages = RefreshKey(
      onRefresh: destinations[_selected].refresh,
      child: IndexedStack(
        index: _selected,
        children: [for (final d in destinations) d.page],
      ),
    );
    return LayoutBuilder(
      builder: (context, constraints) {
        if (constraints.maxWidth < _railMinWidth) {
          return Scaffold(
            body: SafeArea(child: pages),
            bottomNavigationBar: NavigationBar(
              selectedIndex: _selected,
              onDestinationSelected: _select,
              destinations: [
                for (final d in destinations)
                  NavigationDestination(
                    icon: Icon(d.icon),
                    selectedIcon: Icon(d.selectedIcon),
                    label: d.label,
                  ),
              ],
            ),
          );
        }
        return Scaffold(
          body: SafeArea(
            child: Row(
              children: [
                NavigationRail(
                  selectedIndex: _selected,
                  onDestinationSelected: _select,
                  labelType: NavigationRailLabelType.all,
                  trailing: const QuitButton(),
                  trailingAtBottom: true,
                  destinations: [
                    for (final d in destinations)
                      NavigationRailDestination(
                        icon: Icon(d.icon),
                        selectedIcon: Icon(d.selectedIcon),
                        label: Text(d.label),
                      ),
                  ],
                ),
                const VerticalDivider(width: 1),
                Expanded(child: pages),
              ],
            ),
          ),
        );
      },
    );
  }
}
