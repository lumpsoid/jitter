import 'package:flutter/material.dart';

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
        color: behind ? colors.outline : colors.error,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 4),
        child: Text(
          'LIVE',
          style: style?.copyWith(
            color: behind ? colors.surface : colors.onError,
            fontWeight: FontWeight.bold,
          ),
        ),
      ),
    );
  }
}
