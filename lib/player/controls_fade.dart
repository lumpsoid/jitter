import 'package:material_ui/material_ui.dart';

/// How far the player's controls have faded in: 0 hidden, 1 shown. The
/// controls fade by their colours' alpha, read through [fadedColor], not by
/// an Opacity: opacity draws offscreen, which on Impeller's GLES backend
/// stalled the video for up to 118 ms per fade.
class ControlsFade extends InheritedNotifier<Animation<double>> {
  const new({
    required Animation<double> super.notifier,
    required super.child,
    super.key,
  });

  /// The fade above [context], 1 where there is none.
  static double of(BuildContext context) =>
      context
          .dependOnInheritedWidgetOfExactType<ControlsFade>()
          ?.notifier
          ?.value ??
      1;
}

/// [color] with its alpha scaled by the [ControlsFade] above [context].
Color fadedColor(BuildContext context, Color color) =>
    color.withValues(alpha: color.a * ControlsFade.of(context));
