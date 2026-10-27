import 'dart:async';

import 'package:flutter/material.dart';
import 'package:jitter/src/bindings/bindings.dart';

/// Asks for a time to go to in the VOD, typed as `1:23:45` or `1h23m` and
/// sent with Enter or Go. Rust reads it: the dialog closes once [answers]
/// says the player went there, or shows why not and stays to be fixed.
/// A dialog, not a box in the controls: those hide a few seconds after
/// the last touch, typing or not.
class GoToTimeDialog extends StatefulWidget {
  const new({required this.answers, required this.onSubmit, super.key});

  /// Rust's answers to [onSubmit].
  final Stream<GoToTimeAnswer> answers;

  /// Sends the typed text.
  final ValueChanged<String> onSubmit;

  @override
  State<GoToTimeDialog> createState() => _GoToTimeDialogState();
}

class _GoToTimeDialogState extends State<GoToTimeDialog> {
  final _text = TextEditingController();
  late final StreamSubscription<GoToTimeAnswer> _answers;

  /// Why the last time sent is not one to go to; null until one is not.
  String? _rejected;

  @override
  void initState() {
    super.initState();
    _answers = widget.answers.listen(_onAnswer);
  }

  @override
  void dispose() {
    unawaited(_answers.cancel());
    _text.dispose();
    super.dispose();
  }

  void _onAnswer(GoToTimeAnswer answer) {
    if (!mounted) return;
    switch (answer) {
      case GoToTimeAnswerWent():
        Navigator.of(context).pop();
      case GoToTimeAnswerRejected(:final message):
        setState(() => _rejected = message);
    }
  }

  void _submit() => widget.onSubmit(_text.text);

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Go to time'),
      content: TextField(
        controller: _text,
        autofocus: true,
        keyboardType: TextInputType.datetime,
        decoration: InputDecoration(
          hintText: '1:23:45 or 1h23m',
          errorText: _rejected,
        ),
        onChanged: (_) => setState(() => _rejected = null),
        onSubmitted: (_) => _submit(),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(onPressed: _submit, child: const Text('Go')),
      ],
    );
  }
}
