import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:jitter/chat/chat_entry.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:rinf/rinf.dart';

const _noStatus = ChatStatus(modes: []);

/// The live chat as Rust last sent it: a snapshot folded with the diffs
/// after it, in order. Rust bounds the list (it sends `Evicted`). Lives for
/// the app's lifetime.
class ChatHolder extends ChangeNotifier {
  new({this.sendTheme = _send});

  /// Sends `SetTheme`; tests that render the panel without Rust replace it.
  final void Function(SetTheme) sendTheme;

  StreamSubscription<RustSignalPack<ChatUpdate>>? _subscription;
  bool? _dark;

  /// Whether a live chat is open; false while no live channel plays.
  bool get on => _on;
  bool _on = false;

  /// Oldest first.
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

  /// Folds one update in.
  void apply(ChatUpdate update) {
    switch (update) {
      case ChatUpdateOff():
        _on = false;
        _messages = [];
        _status = _noStatus;
      case ChatUpdateSnapshot(:final messages, :final status):
        _on = true;
        _messages = [for (final m in messages) ChatEntry(m)];
        _status = status;
      case ChatUpdateAppended(:final messages):
        _messages.addAll(messages.map(ChatEntry.new));
      case ChatUpdateEvicted(:final count):
        _messages.removeRange(0, count.clamp(0, _messages.length));
      case ChatUpdateMarked(:final ids):
        final marked = ids.toSet();
        for (final entry in _messages) {
          if (marked.contains(entry.line.id)) entry.deleted = true;
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
