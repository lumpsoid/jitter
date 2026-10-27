import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:path_provider/path_provider.dart';
import 'package:rinf/rinf.dart';

/// Where boot stands, as Rust last reported it.
sealed class BootStatus {
  const new();
}

/// [Boot] sent, no answer yet.
final class BootPending extends BootStatus {
  const new();
}

/// Rust opened the store and runs the hub task.
final class BootReady extends BootStatus {
  const new();
}

/// Rust could not open the store; [message] says why.
final class BootError extends BootStatus {
  const new(this.message);

  final String message;
}

/// Whether Rust has booted. Lives for the app's lifetime.
class BootHolder extends ChangeNotifier {
  StreamSubscription<RustSignalPack<Booted>>? _booted;
  StreamSubscription<RustSignalPack<BootFailed>>? _failed;

  BootStatus get status => _status;
  BootStatus _status = const BootPending();

  /// Subscribes to Rust's answers, then sends the first [Boot].
  Future<void> connect() async {
    _booted = Booted.rustSignalStream.listen((_) => booted());
    _failed = BootFailed.rustSignalStream.listen(
      (pack) => fail(pack.message.message),
    );
    await boot();
  }

  /// Sends [Boot] with the app's documents directory. Also the retry after
  /// a failure; once Rust runs, it answers a repeated [Boot] with [Booted].
  Future<void> boot() async {
    _set(const BootPending());
    try {
      final dir = await getApplicationDocumentsDirectory();
      Boot(dir: dir.path).sendSignalToRust();
    } on Exception catch (error) {
      fail('$error');
    }
  }

  /// Rust answered [Booted].
  void booted() => _set(const BootReady());

  /// Shows [message] instead of the spinner.
  void fail(String message) => _set(BootError(message));

  void _set(BootStatus status) {
    _status = status;
    notifyListeners();
  }

  @override
  void dispose() {
    unawaited(_booted?.cancel());
    unawaited(_failed?.cancel());
    super.dispose();
  }
}
