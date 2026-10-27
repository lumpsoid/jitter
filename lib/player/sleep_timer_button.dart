import 'package:flutter/material.dart';
import 'package:jitter/src/bindings/bindings.dart';

/// The player page's sleep timer menu: Rust's presets, and while a timer
/// is armed the time left Rust sent (filled icon, tooltip, menu header)
/// with a way to turn it off.
class SleepTimerButton extends StatelessWidget {
  const new({
    required this.sleep,
    required this.onArm,
    required this.onTurnOff,
    super.key,
  });

  final PlaybackSleep sleep;

  /// Arms a timer of a preset's minutes.
  final ValueChanged<int> onArm;
  final VoidCallback onTurnOff;

  @override
  Widget build(BuildContext context) {
    final left = sleep.left;
    return PopupMenuButton<VoidCallback>(
      icon: Icon(left == null ? Icons.bedtime_outlined : Icons.bedtime),
      tooltip: left == null ? 'Sleep timer' : 'Sleep timer: $left',
      onSelected: (action) => action(),
      itemBuilder: (context) => [
        if (left != null) ...[
          PopupMenuItem(enabled: false, child: Text(left)),
          const PopupMenuDivider(),
        ],
        for (final preset in sleep.presets)
          PopupMenuItem(
            value: () => onArm(preset.minutes),
            child: Text(preset.label),
          ),
        if (left != null) ...[
          const PopupMenuDivider(),
          PopupMenuItem(value: onTurnOff, child: const Text('Turn off timer')),
        ],
      ],
    );
  }
}
