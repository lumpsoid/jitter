import 'dart:async';

import 'package:flutter/material.dart';
import 'package:jitter/app.dart';
import 'package:jitter/boot/boot_holder.dart';
import 'package:jitter/chat/chat_holder.dart';
import 'package:jitter/favorites/favorites_holder.dart';
import 'package:jitter/player/media_kit_engine.dart';
import 'package:jitter/player/playback_holder.dart';
import 'package:jitter/player/player_command_performer.dart';
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
  MediaKit.ensureInitialized();

  // Playback
  final player = MediaKitEngine();
  PlayerCommandPerformer(player)
      .listen(PlayerCommand.rustSignalStream.map((pack) => pack.message));
  final playback = PlaybackHolder()..connect();

  // Browse
  final browse = BrowseHolder()..connect();
  // Favorites
  final favorites = FavoritesHolder()..connect();
  // Chat
  final chat = ChatHolder()..connect();

  final boot = BootHolder();
  unawaited(boot.connect());

  runApp(
    JitterApp(
      boot: boot,
      // Browse
      browse: browse,
      // Favorites
      favorites: favorites,
      // Playback
      playback: playback,
      player: player,
      // Chat
      chat: chat,
    ),
  );
}
