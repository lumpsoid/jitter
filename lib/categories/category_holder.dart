import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:rinf/rinf.dart';

/// The open category's streams as Rust last sent them: a snapshot folded
/// with the appends after it. Lives for the app's lifetime; each category
/// page [open]s its category in it.
class CategoryHolder extends ChangeNotifier {
  StreamSubscription<RustSignalPack<CategoryUpdate>>? _subscription;

  /// Null from [open] until Rust answers it.
  CategoryState? get state => _state;
  CategoryState? _state;

  /// Subscribes to Rust's updates. There is nothing to ask for until a
  /// page [open]s a category, so no ready signal: `OpenCategory` is it.
  void connect() {
    _subscription = CategoryUpdate.rustSignalStream.listen(
      (pack) => apply(pack.message),
    );
  }

  /// Forgets the category shown before, then asks Rust for category [id]'s
  /// streams.
  void open(String id) {
    _state = null;
    notifyListeners();
    OpenCategory(id: id).sendSignalToRust();
  }

  /// Folds one update in: a snapshot replaces the state, an append adds
  /// its streams after the loaded ones (the pinned favorites stay) and
  /// replaces the footer. Rust only appends to a loaded list.
  void apply(CategoryUpdate update) {
    _state = switch (update) {
      CategoryUpdateSnapshot(:final state) => state,
      CategoryUpdateAppended(:final streams, :final more) => switch (_state) {
        CategoryStateLoaded(:final favorites, streams: final loaded) =>
          CategoryStateLoaded(
            favorites: favorites,
            streams: [...loaded, ...streams],
            more: more,
          ),
        final current => current,
      },
      _ => _state,
    };
    notifyListeners();
  }

  /// Starts the open category over from the first page.
  void refresh() => const RefreshCategory().sendSignalToRust();

  /// Asks for the next page; Rust ignores it while one is loading or when
  /// there is none.
  void next() => const NextCategoryPage().sendSignalToRust();

  @override
  void dispose() {
    unawaited(_subscription?.cancel());
    super.dispose();
  }
}
