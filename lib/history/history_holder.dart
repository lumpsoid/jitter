import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:rinf/rinf.dart';

/// The watch history as Rust last sent it. Lives for the app's lifetime:
/// Rust records visits while the player plays, so a snapshot can arrive
/// with the history page out of sight. A removal is sent to Rust, which
/// answers with the history as it now is.
class HistoryHolder extends ChangeNotifier {
  new({
    this.sendRemoveVisit = _sendRemoveVisit,
    this.sendRemoveFromHistory = _sendRemoveFromHistory,
    this.sendClearHistory = _sendClearHistory,
  });

  /// Send the removals; tests without Rust replace them.
  final void Function(RemoveVisit) sendRemoveVisit;
  final void Function(RemoveFromHistory) sendRemoveFromHistory;
  final void Function(ClearHistory) sendClearHistory;

  StreamSubscription<RustSignalPack<HistoryUpdate>>? _subscription;
  final _failures = StreamController<String>.broadcast(sync: true);

  /// One message per removal Rust could not store, for a snackbar each.
  /// Not state: the history is as last sent.
  Stream<String> get failures => _failures.stream;

  /// Null until the first snapshot arrives. The timeline by day, newest
  /// first, and the latest visit to each live channel or VOD.
  HistoryUpdateSnapshot? get history => _history;
  HistoryUpdateSnapshot? _history;

  /// Subscribes to Rust's updates, then asks for the current history.
  void connect() {
    _subscription = HistoryUpdate.rustSignalStream.listen(
      (pack) => apply(pack.message),
    );
    const HistoryReady().sendSignalToRust();
  }

  /// Removes the one visit [entry] stands for.
  void removeVisit(HistoryEntry entry) =>
      sendRemoveVisit(RemoveVisit(id: entry.id));

  /// Removes every visit to what [entry] played.
  void removeFromHistory(HistoryEntry entry) =>
      sendRemoveFromHistory(RemoveFromHistory(target: entry.target));

  /// Removes every visit.
  void clear() => sendClearHistory(const ClearHistory());

  /// Folds one update in: a snapshot replaces the history; a failure
  /// leaves it alone and goes out on [failures].
  void apply(HistoryUpdate update) {
    if (update is HistoryUpdateFailed) {
      _failures.add(update.message);
      return;
    }
    if (update is HistoryUpdateSnapshot) {
      _history = update;
      notifyListeners();
    }
  }

  @override
  void dispose() {
    unawaited(_subscription?.cancel());
    unawaited(_failures.close());
    super.dispose();
  }
}

void _sendRemoveVisit(RemoveVisit signal) => signal.sendSignalToRust();

void _sendRemoveFromHistory(RemoveFromHistory signal) =>
    signal.sendSignalToRust();

void _sendClearHistory(ClearHistory signal) => signal.sendSignalToRust();
