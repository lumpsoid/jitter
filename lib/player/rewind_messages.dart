import 'dart:async';

import 'package:flutter/material.dart';
import 'package:jitter/src/bindings/bindings.dart';

/// How long a message stays over the video.
const _showFor = Duration(seconds: 4);

const _backdrop = Color(0xCC000000);
const _white = Color(0xFFFFFFFF);

/// Shows a short message at the top of the video for a while: each of
/// Rust's [answers] once, and whatever [show] is told, such as why a rewind
/// isn't possible. A new message replaces the one showing.
///
/// It sits in the controls' own stack, around them: media_kit's fullscreen
/// route builds the controls anew and has no `Scaffold` for a `SnackBar`,
/// and its button bar is unmounted while hidden, which would miss an answer
/// arriving then.
class RewindMessages extends StatefulWidget {
  const new({required this.answers, required this.child, super.key});

  /// Rust's answers to a rewind or a return to live: one stream per mount.
  final Stream<RewindAnswer> Function() answers;

  final Widget child;

  /// Shows [message] over the video around [context], if any.
  static void show(BuildContext context, String message) =>
      context.findAncestorStateOfType<_RewindMessagesState>()?._show(message);

  @override
  State<RewindMessages> createState() => _RewindMessagesState();
}

class _RewindMessagesState extends State<RewindMessages> {
  late final StreamSubscription<RewindAnswer> _answers;
  String? _message;
  Timer? _hiding;

  @override
  void initState() {
    super.initState();
    _answers = widget.answers().listen((answer) => _show(answer.message));
  }

  @override
  void dispose() {
    _hiding?.cancel();
    unawaited(_answers.cancel());
    super.dispose();
  }

  void _show(String message) {
    _hiding?.cancel();
    setState(() => _message = message);
    _hiding = Timer(_showFor, () => setState(() => _message = null));
  }

  @override
  Widget build(BuildContext context) {
    final message = _message;
    return Stack(
      fit: StackFit.expand,
      children: [
        widget.child,
        if (message != null)
          Positioned(
            top: 16,
            left: 16,
            right: 16,
            child: IgnorePointer(
              child: Center(child: _Message(text: message)),
            ),
          ),
      ],
    );
  }
}

class _Message extends StatelessWidget {
  const new({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: _backdrop,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
        child: Text(
          text,
          textAlign: TextAlign.center,
          style: const TextStyle(color: _white, fontSize: 14),
        ),
      ),
    );
  }
}
