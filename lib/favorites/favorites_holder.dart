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

  /// Null until the first snapshot arrives. In list order: live by
  /// viewers, then the rest by name.
  List<FavoriteRow>? get favorites => _favorites;
  List<FavoriteRow>? _favorites;

  /// Whether the channel [channelId] is in the list.
  bool isFavorite(String channelId) =>
      _favorites?.any((f) => f.channelId == channelId) ?? false;

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
      _favorites = List.of(update.favorites);
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
