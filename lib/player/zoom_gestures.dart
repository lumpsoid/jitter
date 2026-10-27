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
  /// The pinch's scale at the last update, to zoom by each step of it.
  double _pinched = 1;

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
      child: GestureDetector(
        onScaleStart: (_) => _pinched = 1,
        onScaleUpdate: _onScaleUpdate,
        child: widget.child,
      ),
    );
  }
}
