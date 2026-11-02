import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

/// Ctrl+R refreshes [child]'s page: [onRefresh], when there is one. Only
/// while the route is on top, so a page pushed above it, a dialog or a
/// menu has the key instead.
///
/// Read ahead of the focused widget, as a key handler, as
/// `EscapeGoesBack` reads Escape: a page refreshes wherever its focus is,
/// also when nothing on it has the focus yet.
class RefreshKey extends StatefulWidget {
  const new({required this.child, this.onRefresh, super.key});

  final Widget child;

  /// `null`: the page has nothing to refresh, and the key goes on.
  final VoidCallback? onRefresh;

  @override
  State<RefreshKey> createState() => _RefreshKeyState();
}

class _RefreshKeyState extends State<RefreshKey> {
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
    final keyboard = HardwareKeyboard.instance;
    final onRefresh = widget.onRefresh;
    if (onRefresh == null ||
        event is! KeyDownEvent ||
        event.logicalKey != LogicalKeyboardKey.keyR ||
        !keyboard.isControlPressed ||
        keyboard.isShiftPressed ||
        keyboard.isAltPressed ||
        keyboard.isMetaPressed ||
        !(ModalRoute.of(context)?.isCurrent ?? false)) {
      return false;
    }
    onRefresh();
    return true;
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
