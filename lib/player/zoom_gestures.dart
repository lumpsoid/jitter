import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:jitter/player/video_zoom.dart';

/// Zooms the video it covers through [zoom]: Ctrl and the wheel zoom about
/// the pointer, a pinch about its middle, and a drag moves the zoomed
/// picture. The wheel alone is left to [onWheel], up as positive.
/// The frame is the box this fills; the picture is assumed to fill it.
class ZoomGestures extends StatefulWidget {
  const new({
    required this.zoom,
    required this.onWheel,
    required this.child,
    super.key,
  });

  final ValueNotifier<VideoZoom> zoom;
  final ValueChanged<int> onWheel;
  final Widget child;

  @override
  State<ZoomGestures> createState() => _ZoomGesturesState();
}

/// Keeps the wheel over [child] from [ZoomGestures]: it claims each wheel
/// event before they can, and leaves it to [child]'s own handling
/// (media_kit's volume button sets the volume on it directly).
class KeepsWheel extends StatelessWidget {
  const new({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Listener(
      onPointerSignal: (event) {
        if (event is PointerScrollEvent) {
          GestureBinding.instance.pointerSignalResolver.register(event, (_) {});
        }
      },
      child: child,
    );
  }
}

class _ZoomGesturesState extends State<ZoomGestures> {
  /// The trackpad pinch's scale at the last update, to zoom by each step
  /// of it.
  double _pinched = 1;

  /// The pointers down on the video, where each last was: one drags the
  /// picture, two pinch it.
  final _down = <int, Offset>{};

  /// Where the one pointer down went down, until it has moved past the
  /// hit slop: a tap's wobble does not move the picture. Null while it
  /// drags, or while none or two are down.
  Offset? _dragFrom;

  void _onDown(PointerDownEvent event) {
    _down[event.pointer] = event.localPosition;
    _dragFrom = _down.length == 1 ? event.localPosition : null;
  }

  /// The finger left after a pinch starts over from the slop, so lifting
  /// the other does not jolt the picture.
  void _onUp(PointerEvent event) {
    _down.remove(event.pointer);
    _dragFrom = _down.length == 1 ? _down.values.single : null;
  }

  Offset _fraction(Offset offset) {
    final size = context.size ?? Size.zero;
    if (size.isEmpty) return Offset.zero;
    return Offset(offset.dx / size.width, offset.dy / size.height);
  }

  void _onWheel(PointerSignalEvent event) {
    if (event is! PointerScrollEvent || event.scrollDelta.dy == 0) return;
    GestureBinding.instance.pointerSignalResolver.register(event, (_) {
      final up = event.scrollDelta.dy < 0;
      if (HardwareKeyboard.instance.isControlPressed) {
        widget.zoom.value = widget.zoom.value.zoomBy(
          up ? zoomStep : 1 / zoomStep,
          focus: _fraction(event.localPosition),
        );
      } else {
        widget.onWheel(up ? 1 : -1);
      }
    });
  }

  /// A drag by one pointer or a pinch by two, from their raw moves rather
  /// than gestures: media_kit's mobile controls always take a one-finger
  /// drag in the gesture arena, even with their drag gestures off, and
  /// take it before a pinch would win there.
  void _onMove(PointerMoveEvent event) {
    if (!_down.containsKey(event.pointer)) return;
    final before = _down.values.toList();
    _down[event.pointer] = event.localPosition;
    switch (_down.length) {
      case 1:
        _drag(event);
      case 2:
        _pinch(before, _down.values.toList());
    }
  }

  void _drag(PointerMoveEvent event) {
    final from = _dragFrom;
    if (from != null) {
      final slop = computeHitSlop(
        event.kind,
        MediaQuery.maybeGestureSettingsOf(context),
      );
      // The move past the slop starts the drag; the picture follows from
      // there, not jumping by the slop.
      if ((event.localPosition - from).distance > slop) _dragFrom = null;
      return;
    }
    widget.zoom.value = widget.zoom.value.panBy(_fraction(event.localDelta));
  }

  /// Two fingers moved [from] to [to]: zooms by how far apart they went,
  /// about their middle, and moves with that middle.
  void _pinch(List<Offset> from, List<Offset> to) {
    final span = (from[0] - from[1]).distance;
    if (span == 0) return;
    final factor = (to[0] - to[1]).distance / span;
    final focus = (to[0] + to[1]) / 2;
    final pan = focus - (from[0] + from[1]) / 2;
    widget.zoom.value = widget.zoom.value
        .zoomBy(factor, focus: _fraction(focus))
        .panBy(_fraction(pan));
  }

  /// A trackpad's two fingers; a touch pinch is [_onMove]'s.
  void _onScaleUpdate(ScaleUpdateDetails details) {
    final factor = details.scale / _pinched;
    _pinched = details.scale;
    widget.zoom.value = widget.zoom.value
        .zoomBy(factor, focus: _fraction(details.localFocalPoint))
        .panBy(_fraction(details.focalPointDelta));
  }

  @override
  Widget build(BuildContext context) {
    return Listener(
      onPointerSignal: _onWheel,
      onPointerDown: _onDown,
      onPointerMove: _onMove,
      onPointerUp: _onUp,
      onPointerCancel: _onUp,
      child: GestureDetector(
        supportedDevices: const {PointerDeviceKind.trackpad},
        onScaleStart: (_) => _pinched = 1,
        onScaleUpdate: _onScaleUpdate,
        child: widget.child,
      ),
    );
  }
}
