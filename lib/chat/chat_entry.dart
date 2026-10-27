import 'package:jitter/src/bindings/bindings.dart';

/// One entry in the chat holder: a message, or a user notice (Twitch's own
/// line about a sub, a raid or a watch streak, and what the user wrote with
/// it, if anything), as Rust sent it; and whether a later `Marked` deleted
/// its message. Mutable, so a view holding on to entries (the paused list)
/// still sees a deletion.
class ChatEntry {
  new(this.item) : deleted = _lineOf(item)?.deleted ?? false;

  final ChatItem item;

  /// Whether the message was deleted; a notice without one has nothing to
  /// delete.
  bool deleted;

  /// What the user wrote; null for a notice without text.
  ChatLine? get line => _lineOf(item);

  /// Twitch's line for a user notice; null for a plain message.
  String? get system => switch (item) {
    ChatItemNotice(:final system) => system,
    _ => null,
  };
}

ChatLine? _lineOf(ChatItem item) => switch (item) {
  ChatItemMessage(:final line) => line,
  ChatItemNotice(:final line) => line,
  _ => null,
};
