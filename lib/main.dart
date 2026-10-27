import 'dart:async';

import 'package:flutter/material.dart';
import 'package:jitter/app.dart';
import 'package:jitter/boot/boot_holder.dart';
import 'package:jitter/channel/channel_holder.dart';
import 'package:jitter/favorites/favorites_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:jitter/top/browse_holder.dart';
import 'package:rinf/rinf.dart';

/// Starts Rust, subscribes every holder (each then sends its ready signal,
/// which Rust queues until it has booted), then boots Rust. The GUI shows
/// a spinner until Rust answers [Booted].
Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await initializeRust(assignRustSignal);

  // Browse
  final browse = BrowseHolder()..connect();
  // Favorites
  final favorites = FavoritesHolder()..connect();
  // Channel
  final channel = ChannelHolder()..connect();

  final boot = BootHolder();
  unawaited(boot.connect());

  runApp(
    JitterApp(
      boot: boot,
      // Browse
      browse: browse,
      // Favorites
      favorites: favorites,
      // Channel
      channel: channel,
    ),
  );
}
