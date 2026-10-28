import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:jitter/player/video_zoom.dart';

/// How far down a finger goes for a swipe.
const double _swipeDistance = 100;

/// How far a swipe may go sideways, per its way down: a steep one only,
/// so a drag along the seek bar is not one.
const double _swipeSlant = 0.5;

/// Calls [onSwipe] when one finger goes down [child], steeply, and lifts.
/// Only while [zoom] is none: zoomed, a drag moves the picture. From raw
/// pointer moves, as `ZoomGestures` reads its drag: media_kit's mobile
/// controls take a one-finger drag in the gesture arena.
class SwipeDown extends StatefulWidget {
  const new({
    required this.zoom,
    required this.onSwipe,
    required this.child,
    super.key,
  });

  final ValueListenable<VideoZoom> zoom;
  final VoidCallback onSwipe;
  final Widget child;

  @override
  State<SwipeDown> createState() => _SwipeDownState();
}

class _SwipeDownState extends State<SwipeDown> {
  /// The fingers down, each where it went down.
  final _from = <int, Offset>{};

  /// Whether a second finger came down since the first: a pinch, not a
  /// swipe.
  bool _pinched = false;

  void _onDown(PointerDownEvent event) {
    if (event.kind != PointerDeviceKind.touch) return;
    _from[event.pointer] = event.localPosition;
    _pinched = _from.length > 1;
  }

  void _onUp(PointerUpEvent event) {
    final from = _from.remove(event.pointer);
    if (from == null) return;
    final swiped = !_pinched && widget.zoom.value == VideoZoom.none;
    if (_from.isEmpty) _pinched = false;
    final moved = event.localPosition - from;
    if (swiped &&
        moved.dy >= _swipeDistance &&
        moved.dx.abs() <= moved.dy * _swipeSlant) {
      widget.onSwipe();
    }
  }

  void _onCancel(PointerCancelEvent event) {
    _from.remove(event.pointer);
    if (_from.isEmpty) _pinched = false;
  }

  @override
  Widget build(BuildContext context) {
    return Listener(
      onPointerDown: _onDown,
      onPointerUp: _onUp,
      onPointerCancel: _onCancel,
      child: widget.child,
    );
  }
}
