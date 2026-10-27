import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:media_kit_video/media_kit_video.dart';

/// How long the exit button stays after the pointer last moved or tapped.
const _exitButtonFor = Duration(seconds: 3);

/// Puts the screen into theatre: the window fullscreen on desktop
/// (media_kit's native fullscreen), the system bars hidden on mobile
/// (without media_kit's landscape lock, so the chat can stay below).
Future<void> enterTheatreScreen() => switch (defaultTargetPlatform) {
  TargetPlatform.android || TargetPlatform.iOS =>
    SystemChrome.setEnabledSystemUIMode(SystemUiMode.immersiveSticky),
  _ => defaultEnterNativeFullscreen(),
};

/// Takes the screen out of theatre: the window back from fullscreen, or
/// the system bars back.
Future<void> exitTheatreScreen() => switch (defaultTargetPlatform) {
  TargetPlatform.android || TargetPlatform.iOS =>
    SystemChrome.setEnabledSystemUIMode(SystemUiMode.edgeToEdge),
  _ => defaultExitNativeFullscreen(),
};

/// Leaves media_kit's fullscreen back into theatre: its landscape lock
/// lifted, the screen still in theatre.
Future<void> returnToTheatreScreen() async {
  await SystemChrome.setPreferredOrientations([]);
  await enterTheatreScreen();
}

/// The player page's body, which theatre gives the whole screen while
/// [on]: it puts the screen into theatre while on and takes it out when
/// turned off or disposed (the page left). While on, an exit button shows
/// over the top-left corner for a moment after the pointer moves or taps,
/// and Escape and Back call [onExit]. The button comes first on Tab, and
/// stays while it has the focus.
class Theatre extends StatefulWidget {
  const new({
    required this.on,
    required this.onExit,
    required this.child,
    super.key,
  });

  final bool on;
  final VoidCallback onExit;
  final Widget child;

  @override
  State<Theatre> createState() => _TheatreState();
}

class _TheatreState extends State<Theatre> {
  Timer? _hideExitButton;
  bool _exitButtonShown = false;

  /// Whether the exit button has the focus, which keeps it shown.
  bool _exitButtonFocused = false;

  @override
  void initState() {
    super.initState();
    HardwareKeyboard.instance.addHandler(_onKey);
    if (widget.on) _enter();
  }

  @override
  void didUpdateWidget(Theatre oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.on == oldWidget.on) return;
    if (widget.on) {
      _enter();
    } else {
      _exit();
    }
  }

  @override
  void dispose() {
    HardwareKeyboard.instance.removeHandler(_onKey);
    _hideExitButton?.cancel();
    if (widget.on) unawaited(exitTheatreScreen());
    super.dispose();
  }

  void _enter() {
    unawaited(enterTheatreScreen());
    _showExitButton();
  }

  void _exit() {
    unawaited(exitTheatreScreen());
    _hideExitButton?.cancel();
    _exitButtonShown = false;
    _exitButtonFocused = false;
  }

  void _showExitButton() {
    _hideExitButton?.cancel();
    _hideExitButton = Timer(_exitButtonFor, () {
      if (mounted && !_exitButtonFocused) {
        setState(() => _exitButtonShown = false);
      }
    });
    if (!_exitButtonShown) setState(() => _exitButtonShown = true);
  }

  void _onPointer(PointerEvent _) {
    if (widget.on) _showExitButton();
  }

  /// Leaving the exit button gives it its moment again before it hides.
  void _onExitButtonFocus(bool focused) {
    _exitButtonFocused = focused;
    if (!focused && widget.on) _showExitButton();
  }

  /// Escape, ahead of the focused widget: media_kit's video takes Escape
  /// for its own fullscreen even when not in it. Only while this page is
  /// on top, so Escape on media_kit's fullscreen route leaves that.
  bool _onKey(KeyEvent event) {
    if (!widget.on ||
        event is! KeyDownEvent ||
        event.logicalKey != LogicalKeyboardKey.escape ||
        !(ModalRoute.of(context)?.isCurrent ?? true)) {
      return false;
    }
    widget.onExit();
    return true;
  }

  @override
  Widget build(BuildContext context) {
    final on = widget.on;
    return PopScope(
      canPop: !on,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop) widget.onExit();
      },
      child: MouseRegion(
        onHover: _onPointer,
        child: Listener(
          onPointerDown: _onPointer,
          behavior: HitTestBehavior.translucent,
          child: SafeArea(
            left: on,
            top: on,
            right: on,
            bottom: on,
            child: FocusTraversalGroup(
              policy: OrderedTraversalPolicy(),
              child: Stack(
                children: [
                  Positioned.fill(
                    child: FocusTraversalOrder(
                      order: const NumericFocusOrder(2),
                      child: FocusTraversalGroup(child: widget.child),
                    ),
                  ),
                  if (on && _exitButtonShown)
                    Positioned(
                      top: 8,
                      left: 8,
                      child: FocusTraversalOrder(
                        order: const NumericFocusOrder(1),
                        child: Focus(
                          canRequestFocus: false,
                          skipTraversal: true,
                          includeSemantics: false,
                          onFocusChange: _onExitButtonFocus,
                          child: IconButton.filledTonal(
                            tooltip: 'Exit theatre mode',
                            icon: const Icon(Icons.fullscreen_exit),
                            onPressed: widget.onExit,
                          ),
                        ),
                      ),
                    ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
