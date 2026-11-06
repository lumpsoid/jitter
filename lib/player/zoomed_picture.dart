import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:jitter/player/video_zoom.dart';

/// [child], the video's picture, zoomed by [zoom] within its box. Flutter
/// composites the zoom, so a drag or pinch moves the picture at the
/// screen's rate; libmpv only draws the frames.
class ZoomedPicture extends StatelessWidget {
  const new({required this.zoom, required this.child, super.key});

  final ValueListenable<VideoZoom> zoom;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return ClipRect(
      child: LayoutBuilder(
        builder: (context, constraints) => ValueListenableBuilder(
          valueListenable: zoom,
          builder: (context, value, child) => Transform(
            transform: value.transformIn(constraints.biggest),
            child: child,
          ),
          child: child,
        ),
      ),
    );
  }
}
