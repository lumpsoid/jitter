import 'package:jitter/player/controls_fade.dart';
import 'package:material_ui/material_ui.dart';

/// Marks a stream as live: red at the live edge, grey when [behind] it, as
/// on YouTube.
class LiveBadge extends StatelessWidget {
  const new({this.style, this.behind = false, super.key});

  final TextStyle? style;

  /// Whether what plays is behind the live edge: a rewound stream.
  final bool behind;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: fadedColor(context, behind ? colors.outline : colors.error),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 4),
        child: Text(
          'LIVE',
          style: style?.copyWith(
            color: fadedColor(
              context,
              behind ? colors.surface : colors.onError,
            ),
            fontWeight: FontWeight.bold,
          ),
        ),
      ),
    );
  }
}
