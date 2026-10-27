import 'package:flutter/material.dart';
import 'package:jitter/widgets/retry_failure.dart';

/// Under a paged list: a spinner while the next page loads, its failure with
/// a Retry, otherwise nothing.
class NextPageFooter extends StatelessWidget {
  const new idle({super.key}) : _content = const SizedBox.shrink();

  const new loading({super.key})
    : _content = const Center(child: CircularProgressIndicator());

  new failed({
    required String message,
    required VoidCallback onRetry,
    super.key,
  }) : _content = RetryFailure(message: message, onRetry: onRetry);

  final Widget _content;

  @override
  Widget build(BuildContext context) {
    return Padding(padding: const EdgeInsets.only(bottom: 16), child: _content);
  }
}
