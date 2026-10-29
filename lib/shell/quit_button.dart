import 'dart:async';
import 'dart:io';
import 'dart:ui' show AppExitType;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

/// Whether this is a desktop, where an app quits itself; a phone's system
/// closes apps instead.
final bool _shown = Platform.isLinux || Platform.isWindows || Platform.isMacOS;

/// A button that quits the app on desktop, through the same exit request as
/// the window's close (so the app's exit handler stops the player and Rust
/// first); elsewhere nothing.
class QuitButton extends StatelessWidget {
  const new({super.key});

  @override
  Widget build(BuildContext context) {
    if (!_shown) return const SizedBox.shrink();
    return IconButton(
      tooltip: 'Quit',
      icon: const Icon(Icons.power_settings_new),
      onPressed: () => unawaited(
        ServicesBinding.instance.exitApplication(AppExitType.cancelable),
      ),
    );
  }
}
