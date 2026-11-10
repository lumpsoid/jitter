import 'dart:async';

import 'package:flutter/material.dart';
import 'package:jitter/backup/backup_holder.dart';
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
import 'package:jitter/shell/home_panes.dart';
import 'package:jitter/shell/quit_button.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:jitter/top/browse_holder.dart';
import 'package:jitter/top/top_page.dart';
import 'package:jitter/widgets/escape_goes_back.dart';
import 'package:jitter/widgets/refresh_key.dart';

/// Below this width the destinations sit in a bottom bar, from it in a
/// side rail (Material's compact window class).
const double _railMinWidth = 600;

/// From this width the first [_homePaneCount] destinations (Top,
/// Categories, Favorites) sit side by side under one Home destination, each
/// about as wide as a 1080p window's page.
const double _homeMinWidth = 2700;

/// How many of the first destinations share Home in a window from
/// [_homeMinWidth] wide.
const int _homePaneCount = 3;

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
    required this.backup,
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
  final BackupHolder backup;
  final LogsHolder logs;

  @override
  State<AppShell> createState() => _AppShellState();
}

class _AppShellState extends State<AppShell> {
  int _selected = 0;

  /// One key per destination's page, in [_destinations]' order.
  late final List<GlobalKey> _pageKeys = [
    for (final _ in _destinations) GlobalKey(),
  ];

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
      page: SettingsPage(
        settings: widget.settings,
        backup: widget.backup,
        logs: widget.logs,
      ),
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
    // Each page keeps its state (its scroll position) when a resize moves
    // it between its own place and a Home pane.
    final pages = [
      for (var i = 0; i < destinations.length; i++)
        KeyedSubtree(key: _pageKeys[i], child: destinations[i].page),
    ];
    return LayoutBuilder(
      builder: (context, constraints) {
        final width = constraints.maxWidth;
        if (width < _railMinWidth) {
          return Scaffold(
            body: SafeArea(child: _pages(destinations, pages)),
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
        if (width < _homeMinWidth) {
          return _railed(
            selected: _selected,
            onSelected: _select,
            destinations: [for (final d in destinations) _railEntry(d)],
            body: _pages(destinations, pages),
          );
        }
        final home = _selected < _homePaneCount;
        return _railed(
          selected: home ? 0 : _selected - _homePaneCount + 1,
          onSelected: (index) {
            // Home keeps the pane last selected, for a narrower window.
            if (index == 0 && home) return;
            _select(index == 0 ? 0 : index + _homePaneCount - 1);
          },
          destinations: [
            const NavigationRailDestination(
              icon: Icon(Icons.home_outlined),
              selectedIcon: Icon(Icons.home),
              label: Text('Home'),
            ),
            for (final d in destinations.skip(_homePaneCount)) _railEntry(d),
          ],
          body: RefreshKey(
            onRefresh: home ? null : destinations[_selected].refresh,
            child: IndexedStack(
              index: home ? 0 : _selected - _homePaneCount + 1,
              children: [
                HomePanes(
                  active: home,
                  panes: [
                    for (var i = 0; i < _homePaneCount; i++)
                      (
                        title: destinations[i].label,
                        page: pages[i],
                        refresh: destinations[i].refresh,
                      ),
                  ],
                ),
                ...pages.skip(_homePaneCount),
              ],
            ),
          ),
        );
      },
    );
  }

  /// [pages] one at a time, the selected one shown. Keeps each page alive,
  /// so switching back keeps its scroll position; Ctrl+R refreshes the one
  /// shown.
  Widget _pages(List<_Destination> destinations, List<Widget> pages) =>
      RefreshKey(
        onRefresh: destinations[_selected].refresh,
        child: IndexedStack(index: _selected, children: pages),
      );

  NavigationRailDestination _railEntry(_Destination d) =>
      NavigationRailDestination(
        icon: Icon(d.icon),
        selectedIcon: Icon(d.selectedIcon),
        label: Text(d.label),
      );

  /// [body] beside a side rail of [destinations], the [selected]th
  /// highlighted.
  Widget _railed({
    required int selected,
    required ValueChanged<int> onSelected,
    required List<NavigationRailDestination> destinations,
    required Widget body,
  }) {
    return Scaffold(
      body: SafeArea(
        child: Row(
          children: [
            NavigationRail(
              selectedIndex: selected,
              onDestinationSelected: onSelected,
              labelType: NavigationRailLabelType.all,
              trailing: const QuitButton(),
              trailingAtBottom: true,
              destinations: destinations,
            ),
            const VerticalDivider(width: 1),
            Expanded(child: body),
          ],
        ),
      ),
    );
  }
}
