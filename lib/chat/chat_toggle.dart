import 'package:flutter/material.dart';
import 'package:jitter/chat/chat_holder.dart';
import 'package:jitter/chat/chat_mode.dart';
import 'package:jitter/src/bindings/bindings.dart';

/// The player page's app bar action that hides or shows the chat panel;
/// nothing while there is no chat. Sends `HideChat` / `ShowChat`; Rust
/// leaves or rejoins the chat and answers with the new mode.
class ChatToggle extends StatelessWidget {
  const new({
    required this.chat,
    this.onHide = _hide,
    this.onShow = _show,
    super.key,
  });

  final ChatHolder chat;

  /// Send the intents; tests that render the toggle without Rust replace
  /// them.
  final VoidCallback onHide;
  final VoidCallback onShow;

  @override
  Widget build(BuildContext context) => switch (chat.mode) {
    ChatMode.off => const SizedBox.shrink(),
    ChatMode.shown => IconButton(
      icon: const Icon(Icons.chat_bubble),
      tooltip: 'Hide chat',
      onPressed: onHide,
    ),
    ChatMode.hidden => IconButton(
      icon: const Icon(Icons.chat_bubble_outline),
      tooltip: 'Show chat',
      onPressed: onShow,
    ),
  };
}

void _hide() => const HideChat().sendSignalToRust();
void _show() => const ShowChat().sendSignalToRust();
