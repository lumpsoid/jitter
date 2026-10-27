import 'package:flutter/material.dart';

/// How thick the ring is drawn.
const double _ringWidth = 2;

/// A ring in the theme's primary colour drawn over [child] while [shown],
/// so a keyboard user sees which area has the focus. Drawn in front, so it
/// shows over a video too.
class FocusRing extends StatelessWidget {
  const new({required this.shown, required this.child, super.key});

  final bool shown;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      position: DecorationPosition.foreground,
      decoration: BoxDecoration(
        border: shown
            ? Border.all(
                color: Theme.of(context).colorScheme.primary,
                width: _ringWidth,
              )
            : null,
      ),
      child: child,
    );
  }
}
