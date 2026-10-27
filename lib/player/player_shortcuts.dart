import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

/// The player page's keys, wherever on the page the focus is: `c` the
/// chat, `m` mute, `f` fullscreen, `t` theatre, Space play/pause. Not while a text
/// field has the focus, so it can be typed in. A key the focused widget
/// handles itself (media_kit's video takes Space and `f`) does not reach
/// these, so it fires once.
class PlayerShortcuts extends StatelessWidget {
  const new({
    required this.onToggleChat,
    required this.onToggleMute,
    required this.onToggleFullscreen,
    required this.onTogglePlay,
    required this.onToggleTheatre,
    required this.child,
    super.key,
  });

  final VoidCallback onToggleChat;
  final VoidCallback onToggleMute;
  final VoidCallback onToggleFullscreen;
  final VoidCallback onTogglePlay;
  final VoidCallback onToggleTheatre;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Shortcuts(
      shortcuts: {
        const SingleActivator(LogicalKeyboardKey.keyC): _Press(onToggleChat),
        const SingleActivator(LogicalKeyboardKey.keyM): _Press(onToggleMute),
        const SingleActivator(LogicalKeyboardKey.keyF): _Press(
          onToggleFullscreen,
        ),
        const SingleActivator(LogicalKeyboardKey.space): _Press(onTogglePlay),
        const SingleActivator(LogicalKeyboardKey.keyT): _Press(onToggleTheatre),
      },
      child: Actions(
        actions: {_Press: _PressAction()},
        // Holds the focus until something on the page takes it, so the
        // keys work as soon as the page opens.
        child: Focus(autofocus: true, child: child),
      ),
    );
  }
}

class _Press extends Intent {
  const new(this.callback);

  final VoidCallback callback;
}

class _PressAction extends Action<_Press> {
  /// Disabled while a text field has the focus, which lets the key
  /// through to it.
  @override
  bool isEnabled(_Press intent) =>
      primaryFocus?.context?.findAncestorWidgetOfExactType<EditableText>() ==
      null;

  @override
  void invoke(_Press intent) => intent.callback();
}
