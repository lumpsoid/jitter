import 'package:flutter/material.dart';
import 'package:jitter/widgets/retry_failure.dart';

/// A playback failure on the black video box: its [message], legible on
/// black, with a Retry button.
class PlaybackFailure extends StatelessWidget {
  const new({required this.message, required this.onRetry, super.key});

  final String message;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    return DefaultTextStyle.merge(
      style: const TextStyle(color: Colors.white),
      child: RetryFailure(message: message, onRetry: onRetry),
    );
  }
}
