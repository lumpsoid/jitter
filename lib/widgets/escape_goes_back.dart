import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

/// Escape goes back one level from [child]'s route, as the system Back
/// does: [onEscape], or a `maybePop` of the route, so a `PopScope` under
/// it (theatre's) has its say. Only while the route is on top, so Escape
/// on a dialog or a menu above it is theirs.
///
/// Read ahead of the focused widget, as a key handler: media_kit's video
/// takes Escape for its own fullscreen even when not in it. Wrap each
/// pushed page in one; a gamepad's B maps to Escape.
class EscapeGoesBack extends StatefulWidget {
  const new({required this.child, this.onEscape, super.key});

  final Widget child;
  final VoidCallback? onEscape;

  @override
  State<EscapeGoesBack> createState() => _EscapeGoesBackState();
}

class _EscapeGoesBackState extends State<EscapeGoesBack> {
  @override
  void initState() {
    super.initState();
    HardwareKeyboard.instance.addHandler(_onKey);
  }

  @override
  void dispose() {
    HardwareKeyboard.instance.removeHandler(_onKey);
    super.dispose();
  }

  bool _onKey(KeyEvent event) {
    if (event is! KeyDownEvent ||
        event.logicalKey != LogicalKeyboardKey.escape ||
        !(ModalRoute.of(context)?.isCurrent ?? false)) {
      return false;
    }
    final onEscape = widget.onEscape;
    if (onEscape != null) {
      onEscape();
    } else {
      Navigator.maybePop(context);
    }
    return true;
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
