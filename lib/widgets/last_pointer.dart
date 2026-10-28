import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';

/// Whether the last pointer over [child] was a finger (or a stylus) rather
/// than a mouse or a trackpad: a touchscreen laptop or a Steam Deck has
/// both, so the platform alone does not say which controls suit. Until the
/// first pointer it guesses from the platform: touch on Android and iOS.
/// Put it over the navigator, so every route reads the same answer.
class LastPointer extends StatefulWidget {
  const new({required this.child, super.key});

  final Widget child;

  /// Whether the last pointer was a touch; rebuilds [context] when that
  /// changes.
  static bool isTouch(BuildContext context) =>
      context
          .dependOnInheritedWidgetOfExactType<_LastPointerScope>()
          ?.notifier
          ?.value ??
      _guess;

  static bool get _guess => switch (defaultTargetPlatform) {
    TargetPlatform.android || TargetPlatform.iOS => true,
    _ => false,
  };

  @override
  State<LastPointer> createState() => _LastPointerState();
}

class _LastPointerState extends State<LastPointer> {
  final ValueNotifier<bool> _touch = ValueNotifier(LastPointer._guess);

  @override
  void dispose() {
    _touch.dispose();
    super.dispose();
  }

  void _onPointer(PointerEvent event) => _touch.value = switch (event.kind) {
    PointerDeviceKind.touch ||
    PointerDeviceKind.stylus ||
    PointerDeviceKind.invertedStylus => true,
    PointerDeviceKind.mouse || PointerDeviceKind.trackpad => false,
    PointerDeviceKind.unknown => _touch.value,
  };

  @override
  Widget build(BuildContext context) {
    return Listener(
      onPointerDown: _onPointer,
      onPointerHover: _onPointer,
      behavior: HitTestBehavior.translucent,
      child: _LastPointerScope(notifier: _touch, child: widget.child),
    );
  }
}

class _LastPointerScope extends InheritedNotifier<ValueNotifier<bool>> {
  const new({required super.notifier, required super.child});
}
