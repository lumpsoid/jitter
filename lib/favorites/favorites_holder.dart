import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:rinf/rinf.dart';

/// The favorites as Rust last sent them. Lives for the app's lifetime; the
/// favorites page and every `FavoriteStar` read it.
class FavoritesHolder extends ChangeNotifier {
  StreamSubscription<RustSignalPack<FavoritesUpdate>>? _subscription;
  final _failures = StreamController<String>.broadcast(sync: true);
  final _refreshes = <Completer<void>>[];

  /// One message per failed add, remove, reload or live check, for a
  /// snackbar each; repeats included. Not list state: the list is as last
  /// sent.
  Stream<String> get failures => _failures.stream;

  /// Null until the first snapshot arrives. Live by viewers, then offline
  /// and not-yet-checked favorites, each by name; a favorite is in exactly
  /// one of the three.
  FavoritesUpdateSnapshot? get favorites => _favorites;
  FavoritesUpdateSnapshot? _favorites;
  Set<String> _ids = const {};

  /// Whether the snapshot lists no favorite at all.
  bool get isEmpty => _ids.isEmpty;

  /// Whether the channel [channelId] is in the list.
  bool isFavorite(String channelId) => _ids.contains(channelId);

  /// Subscribes to Rust's updates, then asks for the current snapshot.
  void connect() {
    _subscription = FavoritesUpdate.rustSignalStream.listen(
      (pack) => apply(pack.message),
    );
    const FavoritesReady().sendSignalToRust();
  }

  /// Asks Rust to re-read the favorites and check who is live. Completes
  /// with the next snapshot (the reloaded list); live status follows in
  /// a later one.
  Future<void> refresh() {
    final done = Completer<void>();
    _refreshes.add(done);
    const RefreshFavorites().sendSignalToRust();
    return done.future;
  }

  /// Folds one update in: a snapshot replaces the list and ends any
  /// pending [refresh]; a failure leaves the list alone and goes out on
  /// [failures].
  void apply(FavoritesUpdate update) {
    if (update is FavoritesUpdateFailed) {
      _failures.add(update.message);
      return;
    }
    if (update is FavoritesUpdateSnapshot) {
      _favorites = update;
      _ids = {
        for (final f in update.live) f.stream.channelId,
        for (final f in update.offline) f.channelId,
        for (final f in update.unchecked) f.channelId,
      };
      notifyListeners();
      for (final done in _refreshes) {
        done.complete();
      }
      _refreshes.clear();
    }
  }

  @override
  void dispose() {
    unawaited(_subscription?.cancel());
    unawaited(_failures.close());
    super.dispose();
  }
}
