import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:jitter/chat/chat_entry.dart';
import 'package:jitter/chat/chat_mode.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:rinf/rinf.dart';

const _noStatus = ChatStatus(modes: []);

/// The chat (a live channel's, or a VOD's replay) as Rust last sent it: a
/// snapshot folded with the diffs after it, in order. Rust bounds the list
/// (it sends `Evicted`). Lives for the app's lifetime.
class ChatHolder extends ChangeNotifier {
  new({this.sendTheme = _send, this.sendHide = _hide, this.sendShow = _show});

  /// Sends `SetTheme`; tests that render the panel without Rust replace it.
  final void Function(SetTheme) sendTheme;

  /// Send `HideChat` / `ShowChat`; tests without Rust replace them.
  final VoidCallback sendHide;
  final VoidCallback sendShow;

  StreamSubscription<RustSignalPack<ChatUpdate>>? _subscription;
  bool? _dark;

  /// Whether the chat is shown, hidden by the user, or absent.
  ChatMode get mode => _mode;
  ChatMode _mode = ChatMode.off;

  /// Messages and user notices, oldest first.
  List<ChatEntry> get messages => _messages;
  var _messages = <ChatEntry>[];

  ChatStatus get status => _status;
  ChatStatus _status = _noStatus;

  /// Subscribes to Rust's updates, then asks for the current snapshot.
  void connect() {
    _subscription = ChatUpdate.rustSignalStream.listen(
      (pack) => apply(pack.message),
    );
    const ChatReady().sendSignalToRust();
  }

  /// Tells Rust the GUI's brightness, so names are coloured readably on
  /// it. Sent once per change.
  void setDark({required bool dark}) {
    if (dark == _dark) return;
    _dark = dark;
    sendTheme(SetTheme(dark: dark));
  }

  /// Asks Rust to hide a shown chat or show a hidden one; nothing while
  /// there is no chat. Rust leaves or rejoins it and answers with the new
  /// mode.
  void toggle() => switch (_mode) {
    ChatMode.off => null,
    ChatMode.shown => sendHide(),
    ChatMode.hidden => sendShow(),
  };

  /// Folds one update in.
  void apply(ChatUpdate update) {
    switch (update) {
      case ChatUpdateOff():
        _mode = ChatMode.off;
        _messages = [];
        _status = _noStatus;
      case ChatUpdateHidden():
        _mode = ChatMode.hidden;
        _messages = [];
        _status = _noStatus;
      case ChatUpdateSnapshot(:final messages, :final status):
        _mode = ChatMode.shown;
        _messages = [for (final m in messages) ChatEntry(m)];
        _status = status;
      case ChatUpdateAppended(:final messages):
        _messages.addAll(messages.map(ChatEntry.new));
      case ChatUpdateEvicted(:final count):
        _messages.removeRange(0, count.clamp(0, _messages.length));
      case ChatUpdateMarked(:final ids):
        final marked = ids.toSet();
        for (final entry in _messages) {
          if (marked.contains(entry.line?.id)) entry.deleted = true;
        }
      case ChatUpdateCleared():
        _messages = [];
      case ChatUpdateStatus(:final status):
        _status = status;
    }
    notifyListeners();
  }

  @override
  void dispose() {
    unawaited(_subscription?.cancel());
    super.dispose();
  }
}

void _send(SetTheme signal) => signal.sendSignalToRust();
void _hide() => const HideChat().sendSignalToRust();
void _show() => const ShowChat().sendSignalToRust();
