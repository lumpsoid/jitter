import 'dart:async';
import 'dart:ui';

import 'package:audio_service/audio_service.dart';
import 'package:flutter/material.dart';
import 'package:jitter/app.dart';
import 'package:jitter/boot/boot_holder.dart';
import 'package:jitter/categories/categories_holder.dart';
import 'package:jitter/categories/category_holder.dart';
import 'package:jitter/channel/channel_holder.dart';
import 'package:jitter/chat/chat_holder.dart';
import 'package:jitter/favorites/favorites_holder.dart';
import 'package:jitter/history/history_holder.dart';
import 'package:jitter/logs/logs_holder.dart';
import 'package:jitter/logs/rust_log.dart';
import 'package:jitter/player/media_kit_engine.dart';
import 'package:jitter/player/media_session.dart';
import 'package:jitter/player/playback_holder.dart';
import 'package:jitter/player/player_command_performer.dart';
import 'package:jitter/search/search_holder.dart';
import 'package:jitter/settings/settings_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:jitter/top/browse_holder.dart';
import 'package:media_kit/media_kit.dart';
import 'package:rinf/rinf.dart';

/// Starts Rust, subscribes every holder (each then sends its ready signal,
/// which Rust queues until it has booted), then boots Rust. The GUI shows
/// a spinner until Rust answers [Booted].
Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await initializeRust(assignRustSignal);
  _logErrors();
  MediaKit.ensureInitialized();

  // Playback
  final player = MediaKitEngine();
  PlayerCommandPerformer(player)
      .listen(PlayerCommand.rustSignalStream.map((pack) => pack.message));
  final playback = PlaybackHolder()..connect();
  await _startMediaSession(playback, player);

  // Browse
  final browse = BrowseHolder()..connect();
  // Categories
  final categories = CategoriesHolder()..connect();
  final category = CategoryHolder()..connect();
  // Favorites
  final favorites = FavoritesHolder()..connect();
  // History
  final history = HistoryHolder()..connect();
  // Channel
  final channel = ChannelHolder()..connect();
  // Chat
  final chat = ChatHolder()..connect();
  // Search
  final search = SearchHolder()..connect();
  // Settings
  final settings = SettingsHolder()..connect();
  final logs = LogsHolder()..connect();

  final boot = BootHolder();
  unawaited(boot.connect());

  runApp(
    JitterApp(
      boot: boot,
      // Browse
      browse: browse,
      // Categories
      categories: categories,
      category: category,
      // Favorites
      favorites: favorites,
      // History
      history: history,
      // Playback
      playback: playback,
      player: player,
      // Channel
      channel: channel,
      // Chat
      chat: chat,
      // Search
      search: search,
      // Settings
      settings: settings,
      logs: logs,
    ),
  );
}

/// Sends every uncaught error to this run's log, and still reports it as
/// Flutter would.
void _logErrors() {
  FlutterError.onError = (details) {
    FlutterError.presentError(details);
    logToRust(
      DartLogLevel.error,
      'flutter',
      '${details.exceptionAsString()}\n${details.stack}',
    );
  };
  PlatformDispatcher.instance.onError = (error, stack) {
    logToRust(DartLogLevel.error, 'dart', '$error\n$stack');
    return false;
  };
}

/// Shows the playing stream to the system (a no-op on Linux and Windows).
/// A failure is logged: the app plays in front without it.
Future<void> _startMediaSession(
  PlaybackHolder playback,
  MediaKitEngine player,
) async {
  try {
    final session = await AudioService.init(
      builder: () => MediaSession(playback, player),
      config: const AudioServiceConfig(
        androidNotificationChannelId: 'io.github.lumpsoid.jitter.channel.playback',
        androidNotificationChannelName: 'Playback',
        // The default, the launcher icon, is opaque and renders as a blank
        // disc: Android draws a notification icon from its alpha only.
        androidNotificationIcon: 'drawable/ic_stat_jitter',
        // A pause keeps the foreground service: Android 12+ may refuse to
        // start it again from the background.
        androidStopForegroundOnPause: false,
      ),
    );
    session.connect();
  } on Object catch (error) {
    logToRust(DartLogLevel.warning, 'media session', '$error');
  }
}
