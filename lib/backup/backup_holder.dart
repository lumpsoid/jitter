import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:rinf/rinf.dart';

/// Backups of the user's favorites, history, resume points and settings.
/// Rust writes a backup's text and reads it back; the settings page picks
/// the files. Lives for the app's lifetime.
class BackupHolder extends ChangeNotifier {
  new({this.sendExport = _sendExport, this.sendImport = _sendImport});

  /// Send the page's asks; tests without Rust replace them.
  final void Function(ExportBackup) sendExport;
  final void Function(ImportBackup) sendImport;

  StreamSubscription<RustSignalPack<BackupUpdate>>? _subscription;
  final _answers = StreamController<BackupUpdate>.broadcast(sync: true);

  /// Rust's answer to each export or import: one-shot notices, not state.
  Stream<BackupUpdate> get answers => _answers.stream;

  /// Whether an export or import is waiting for Rust's answer.
  bool get busy => _busy;
  bool _busy = false;

  /// Subscribes to Rust's answers.
  void connect() {
    _subscription = BackupUpdate.rustSignalStream.listen(
      (pack) => apply(pack.message),
    );
  }

  /// Asks Rust for a backup; it arrives on [answers] as
  /// [BackupUpdateExported].
  void export() => _ask(() => sendExport(const ExportBackup()));

  /// Hands Rust a backup file's [json] to merge into what is stored.
  void import(String json) => _ask(() => sendImport(ImportBackup(json: json)));

  void _ask(void Function() send) {
    _busy = true;
    notifyListeners();
    send();
  }

  /// Folds one answer in: it ends the wait and goes out on [answers].
  void apply(BackupUpdate update) {
    _busy = false;
    notifyListeners();
    _answers.add(update);
  }

  @override
  void dispose() {
    unawaited(_subscription?.cancel());
    unawaited(_answers.close());
    super.dispose();
  }
}

void _sendExport(ExportBackup signal) => signal.sendSignalToRust();

void _sendImport(ImportBackup signal) => signal.sendSignalToRust();
