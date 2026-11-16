import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:jitter/player/theatre.dart';
import 'package:material_ui/material_ui.dart';
import 'package:media_kit_video/media_kit_video.dart';

/// Puts the screen into fullscreen: the window fullscreen on desktop
/// (media_kit's native fullscreen for now), the system bars hidden and the
/// screen locked to landscape on mobile.
Future<void> enterFullscreenScreen() async {
  switch (defaultTargetPlatform) {
    case TargetPlatform.android || TargetPlatform.iOS:
      await SystemChrome.setEnabledSystemUIMode(SystemUiMode.immersiveSticky);
      await SystemChrome.setPreferredOrientations([
        DeviceOrientation.landscapeLeft,
        DeviceOrientation.landscapeRight,
      ]);
    case _:
      await defaultEnterNativeFullscreen();
  }
}

/// Takes the screen out of fullscreen to the app's normal mode: the window
/// back from fullscreen, or the system bars back and the landscape lock
/// lifted.
Future<void> exitFullscreenScreen() async {
  switch (defaultTargetPlatform) {
    case TargetPlatform.android || TargetPlatform.iOS:
      await SystemChrome.setEnabledSystemUIMode(SystemUiMode.edgeToEdge);
      await SystemChrome.setPreferredOrientations([]);
    case _:
      await defaultExitNativeFullscreen();
  }
}

/// The player page's body, which goes fullscreen while [on]: it puts the
/// screen into fullscreen while on. Turned off, it takes the screen out:
/// back into theatre when fullscreen was entered from theatre ([theatre]),
/// else to the app mode. Disposed (the page left) while on, it always takes
/// the screen to the app mode, theatre included, since the page's theatre
/// goes with it. While on, Back calls [onExit] rather than leaving the page.
/// Widgets under it read the flag with [isOn].
///
/// Fullscreen is in place, a flag like theatre and not a route, so the
/// page's own controls stay where they are.
class Fullscreen extends StatefulWidget {
  const new({
    required this.on,
    required this.theatre,
    required this.onExit,
    required this.child,
    super.key,
  });

  final bool on;

  /// Whether fullscreen was entered from theatre.
  final bool theatre;
  final VoidCallback onExit;
  final Widget child;

  /// Whether the nearest [Fullscreen] is on; false with none above.
  /// Replaces media_kit's `isFullscreen(context)` for widgets under the
  /// player. Rebuilds [context] when that changes.
  static bool isOn(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<_FullscreenScope>()?.on ??
      false;

  @override
  State<Fullscreen> createState() => _FullscreenState();
}

class _FullscreenState extends State<Fullscreen> {
  @override
  void initState() {
    super.initState();
    if (widget.on) _enter();
  }

  @override
  void didUpdateWidget(Fullscreen oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.on == oldWidget.on) return;
    if (widget.on) {
      _enter();
    } else {
      _exit();
    }
  }

  /// The page is gone, theatre with it, so this always takes the screen to
  /// the app mode. Going back into theatre here would finish after the
  /// page's own theatre exit (Flutter unmounts children first, and
  /// [returnToTheatreScreen] awaits the orientation call before it enters
  /// theatre), leaving the bars hidden off the page.
  @override
  void dispose() {
    if (widget.on) unawaited(exitFullscreenScreen());
    super.dispose();
  }

  void _enter() => unawaited(enterFullscreenScreen());

  void _exit() => unawaited(
    widget.theatre ? returnToTheatreScreen() : exitFullscreenScreen(),
  );

  @override
  Widget build(BuildContext context) {
    final on = widget.on;
    return PopScope(
      canPop: !on,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop) widget.onExit();
      },
      child: _FullscreenScope(on: on, child: widget.child),
    );
  }
}

/// Carries the flag down to [Fullscreen.isOn].
class _FullscreenScope extends InheritedWidget {
  const new({required this.on, required super.child});

  final bool on;

  @override
  bool updateShouldNotify(_FullscreenScope oldWidget) => on != oldWidget.on;
}
