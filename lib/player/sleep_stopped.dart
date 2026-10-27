import 'package:flutter/material.dart';

/// The sleep timer stopped the player: said on the black video box, legible
/// on black, with a Resume button that takes the focus. A live region, so a
/// screen reader announces it when it replaces the video.
class SleepStopped extends StatelessWidget {
  const new({required this.onResume, super.key});

  final VoidCallback onResume;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      container: true,
      liveRegion: true,
      child: DefaultTextStyle.merge(
        style: const TextStyle(color: Colors.white),
        child: Center(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                const Text(
                  'Stopped by the sleep timer',
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: 8),
                FilledButton.tonal(
                  // The video it replaced had the focus; its keys go with it.
                  autofocus: true,
                  onPressed: onResume,
                  child: const Text('Resume'),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
