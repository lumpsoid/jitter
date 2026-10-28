import 'dart:ui' show AppExitResponse;

import 'package:flutter/material.dart';
import 'package:jitter/boot/boot_failure_page.dart';
import 'package:jitter/boot/boot_holder.dart';
import 'package:jitter/categories/categories_holder.dart';
import 'package:jitter/categories/category_holder.dart';
import 'package:jitter/channel/channel_holder.dart';
import 'package:jitter/chat/chat_holder.dart';
import 'package:jitter/favorites/favorites_holder.dart';
import 'package:jitter/history/history_holder.dart';
import 'package:jitter/player/media_kit_engine.dart';
import 'package:jitter/player/playback_holder.dart';
import 'package:jitter/search/search_holder.dart';
import 'package:jitter/settings/settings_holder.dart';
import 'package:jitter/shell/app_shell.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:jitter/top/browse_holder.dart';
import 'package:rinf/rinf.dart';

/// The app: a spinner until Rust has booted, its failure with a Retry, or
/// the navigation shell. Feature holders are created in `main` and passed
/// down from here.
class JitterApp extends StatefulWidget {
  const new({
    required this.boot,
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
    super.key,
  });

  final BootHolder boot;
  // Browse
  final BrowseHolder browse;
  // Categories
  final CategoriesHolder categories;
  final CategoryHolder category;
  // Favorites
  final FavoritesHolder favorites;
  // History
  final HistoryHolder history;
  // Playback
  final PlaybackHolder playback;
  final MediaKitEngine player;
  // Channel
  final ChannelHolder channel;
  // Chat
  final ChatHolder chat;
  // Search
  final SearchHolder search;
  // Settings: the theme here, the rest on the settings page.
  final SettingsHolder settings;

  @override
  State<JitterApp> createState() => _JitterAppState();
}

class _JitterAppState extends State<JitterApp> {
  late final AppLifecycleListener _lifecycle;

  @override
  void initState() {
    super.initState();
    _lifecycle = AppLifecycleListener(
      // Lets Rust shut down and close the database before the process ends.
      onExitRequested: () async {
        finalizeRust();
        return AppExitResponse.exit;
      },
      // Rust decides what the player does while the app is out of sight.
      onHide: () =>
          AppBackgrounded(positionS: widget.player.position.inSeconds)
              .sendSignalToRust(),
      onShow: () =>
          AppForegrounded(positionS: widget.player.position.inSeconds)
              .sendSignalToRust(),
    );
  }

  @override
  void dispose() {
    _lifecycle.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: widget.settings,
      builder: (context, home) => MaterialApp(
        title: 'Jitter',
        theme: ThemeData(colorSchemeSeed: Colors.deepPurple),
        darkTheme: ThemeData(
          colorSchemeSeed: Colors.deepPurple,
          brightness: Brightness.dark,
        ),
        themeMode: widget.settings.themeMode,
        home: home,
      ),
      child: ListenableBuilder(
        listenable: widget.boot,
        builder: (context, _) => switch (widget.boot.status) {
          BootPending() => const Scaffold(
            body: Center(child: CircularProgressIndicator()),
          ),
          BootError(:final message) => BootFailurePage(
            message: message,
            onRetry: widget.boot.boot,
          ),
          BootReady() => AppShell(
            // Browse
            browse: widget.browse,
            // Categories
            categories: widget.categories,
            category: widget.category,
            // Favorites
            favorites: widget.favorites,
            // History
            history: widget.history,
            // Playback
            playback: widget.playback,
            player: widget.player,
            // Channel
            channel: widget.channel,
            // Chat
            chat: widget.chat,
            // Search
            search: widget.search,
            // Settings
            settings: widget.settings,
          ),
        },
      ),
    );
  }
}
