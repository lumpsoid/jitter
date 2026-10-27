import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:rinf/rinf.dart';

/// The top streams as Rust last sent them: a snapshot folded with the
/// appends after it. Lives for the app's lifetime.
class BrowseHolder extends ChangeNotifier {
  StreamSubscription<RustSignalPack<BrowseUpdate>>? _subscription;

  /// Null until the first snapshot arrives.
  BrowseState? get state => _state;
  BrowseState? _state;

  /// Subscribes to Rust's updates, then asks for the current snapshot.
  void connect() {
    _subscription = BrowseUpdate.rustSignalStream.listen(
      (pack) => apply(pack.message),
    );
    const BrowseReady().sendSignalToRust();
  }

  /// Folds one update in: a snapshot replaces the state, an append adds
  /// its streams after the loaded ones and replaces the footer. Rust only
  /// appends to a loaded list.
  void apply(BrowseUpdate update) {
    _state = switch (update) {
      BrowseUpdateSnapshot(:final state) => state,
      BrowseUpdateAppended(:final streams, :final more) => switch (_state) {
        BrowseStateLoaded(streams: final loaded) => BrowseStateLoaded(
          streams: [...loaded, ...streams],
          more: more,
        ),
        final current => current,
      },
      _ => _state,
    };
    notifyListeners();
  }

  /// Starts over from the first page.
  void refresh() => const RefreshBrowse().sendSignalToRust();

  /// Asks for the next page; Rust ignores it while one is loading or when
  /// there is none.
  void next() => const NextBrowsePage().sendSignalToRust();

  @override
  void dispose() {
    unawaited(_subscription?.cancel());
    super.dispose();
  }
}
