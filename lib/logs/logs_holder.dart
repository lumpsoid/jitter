import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:rinf/rinf.dart';

/// The logs page as Rust last sent it: the kept sessions, and the one the
/// user opened. Lives for the app's lifetime; the page asks for the list
/// each time it opens, as a session's log grows while it runs.
class LogsHolder extends ChangeNotifier {
  new({this.sendLogsReady = _sendLogsReady, this.sendOpenLog = _sendOpenLog});

  /// Send the page's asks; tests without Rust replace them.
  final void Function(LogsReady) sendLogsReady;
  final void Function(OpenLog) sendOpenLog;

  StreamSubscription<RustSignalPack<LogsUpdate>>? _subscription;
  final _failures = StreamController<String>.broadcast(sync: true);

  /// One message per list or session Rust could not read, for a snackbar
  /// each.
  Stream<String> get failures => _failures.stream;

  /// Null until the first list arrives.
  LogsUpdateSessions? get sessions => _sessions;
  LogsUpdateSessions? _sessions;

  /// The session last opened, null while it loads.
  LogsUpdateOpened? get opened => _opened;
  LogsUpdateOpened? _opened;

  /// [opened]'s text, a line each.
  List<String> get lines => _lines;
  List<String> _lines = const [];

  /// Subscribes to Rust's updates.
  void connect() {
    _subscription = LogsUpdate.rustSignalStream.listen(
      (pack) => apply(pack.message),
    );
  }

  /// Asks for the kept sessions.
  void refresh() => sendLogsReady(const LogsReady());

  /// Asks for session [name]'s log; [opened] is null until it arrives.
  void open(String name) {
    _opened = null;
    _lines = const [];
    notifyListeners();
    sendOpenLog(OpenLog(name: name));
  }

  /// Folds one update in: a list or a session replaces the last one; a
  /// failure leaves them alone and goes out on [failures].
  void apply(LogsUpdate update) {
    switch (update) {
      case LogsUpdateSessions():
        _sessions = update;
      case LogsUpdateOpened(:final text):
        _opened = update;
        _lines = text.isEmpty ? const [] : text.trimRight().split('\n');
      case LogsUpdateFailed(:final message):
        _failures.add(message);
        return;
    }
    notifyListeners();
  }

  @override
  void dispose() {
    unawaited(_subscription?.cancel());
    unawaited(_failures.close());
    super.dispose();
  }
}

void _sendLogsReady(LogsReady signal) => signal.sendSignalToRust();

void _sendOpenLog(OpenLog signal) => signal.sendSignalToRust();
