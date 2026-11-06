import 'package:flutter/widgets.dart';

/// How far one wheel notch or `+`/`-` press zooms in or out.
const double zoomStep = 1.25;

/// The furthest the video zooms in.
const double _maxScale = 8;

const _middle = Offset(0.5, 0.5);

/// A zoom into the video: the picture at [scale] times its fitted size,
/// its point [center] (fractions of it, `(0.5, 0.5)` its middle) at the
/// middle of the frame. The frame never shows past the picture's edges.
@immutable
class VideoZoom {
  const new _(this.scale, this.center);

  /// At [scale], [center] moved in as far as it must be to keep the frame
  /// within the picture.
  factory _clamped(double scale, Offset center) {
    final half = 0.5 / scale;
    return VideoZoom._(
      scale,
      Offset(center.dx.clamp(half, 1 - half), center.dy.clamp(half, 1 - half)),
    );
  }

  /// The whole picture.
  static const none = VideoZoom._(1, _middle);

  final double scale;
  final Offset center;

  bool get zoomed => scale > 1;

  /// Zoomed by [factor], the point of the picture under [focus] (fractions
  /// of the frame) staying under it.
  VideoZoom zoomBy(double factor, {Offset focus = _middle}) {
    final scale = (this.scale * factor).clamp(1.0, _maxScale);
    final fromMiddle = focus - _middle;
    final under = center + fromMiddle / this.scale;
    return VideoZoom._clamped(scale, under - fromMiddle / scale);
  }

  /// Dragged by [delta] (fractions of the frame): the picture moves with it.
  VideoZoom panBy(Offset delta) =>
      VideoZoom._clamped(scale, center - delta / scale);

  /// What draws a [frame]-sized picture zoomed: scaled by [scale], its
  /// [center] moved to the frame's middle.
  Matrix4 transformIn(Size frame) => Matrix4.translationValues(
    (0.5 - center.dx * scale) * frame.width,
    (0.5 - center.dy * scale) * frame.height,
    0,
  )..scaleByDouble(scale, scale, 1, 1);

  @override
  bool operator ==(Object other) =>
      other is VideoZoom && other.scale == scale && other.center == center;

  @override
  int get hashCode => Object.hash(scale, center);

  @override
  String toString() => 'VideoZoom($scale, $center)';
}
