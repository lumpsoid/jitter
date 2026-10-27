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
import 'package:jitter/player/media_kit_engine.dart';
import 'package:jitter/player/playback_holder.dart';
import 'package:jitter/player/player_page.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:jitter/top/browse_holder.dart';
import 'package:jitter/top/top_page.dart';

/// Below this width the destinations sit in a bottom bar, from it in a
/// side rail (Material's compact window class).
const double _railMinWidth = 600;

/// One top-level page in the navigation.
typedef _Destination = ({
  String label,
  IconData icon,
  IconData selectedIcon,
  Widget page,
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
    // Playback
    required this.playback,
    required this.player,
    // Channel
    required this.channel,
    // Chat
    required this.chat,
    super.key,
  });

  // Browse
  final BrowseHolder browse;
  // Categories
  final CategoriesHolder categories;
  final CategoryHolder category;
  // Favorites
  final FavoritesHolder favorites;
  // Playback: what a page needs to push a `PlayerPage`.
  final PlaybackHolder playback;
  final MediaKitEngine player;
  // Channel
  final ChannelHolder channel;
  // Chat: shown on the player page.
  final ChatHolder chat;

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
      ),
    ),
    (
      label: 'Categories',
      icon: Icons.category_outlined,
      selectedIcon: Icons.category,
      page: CategoriesPage(
        categories: widget.categories,
        onOpenCategory: _openCategory,
      ),
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
    ),
  );

  // Channel
  void _openChannel(String login) => unawaited(
    ChannelPage.push(
      context,
      login: login,
      channel: widget.channel,
      favorites: widget.favorites,
      onOpenVod: (id) => _openPlayer(PlaybackTargetVod(id: id), id),
      onOpenLive: _openLive,
    ),
  );

  void _openLive(String login) =>
      _openPlayer(PlaybackTargetLive(login: login), login);

  void _select(int index) => setState(() => _selected = index);

  /// Pushes the player for `target`; leaving the page stops it.
  void _openPlayer(PlaybackTarget target, String title) {
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => PlayerPage(
          target: target,
          title: title,
          playback: widget.playback,
          player: widget.player,
          chat: widget.chat,
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final destinations = _destinations;
    // Keeps each page alive, so switching back keeps its scroll position.
    final pages = IndexedStack(
      index: _selected,
      children: [for (final d in destinations) d.page],
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
