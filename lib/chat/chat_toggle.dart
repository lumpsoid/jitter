import 'package:flutter/material.dart';
import 'package:jitter/chat/chat_holder.dart';
import 'package:jitter/chat/chat_mode.dart';

/// The player page's app bar action that hides or shows the chat panel;
/// nothing while there is no chat. Taps go to [ChatHolder.toggle].
class ChatToggle extends StatelessWidget {
  const new({required this.chat, super.key});

  final ChatHolder chat;

  @override
  Widget build(BuildContext context) => switch (chat.mode) {
    ChatMode.off => const SizedBox.shrink(),
    ChatMode.shown => IconButton(
      icon: const Icon(Icons.chat_bubble),
      tooltip: 'Hide chat',
      onPressed: chat.toggle,
    ),
    ChatMode.hidden => IconButton(
      icon: const Icon(Icons.chat_bubble_outline),
      tooltip: 'Show chat',
      onPressed: chat.toggle,
    ),
  };
}
