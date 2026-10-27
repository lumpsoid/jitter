import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:rinf/rinf.dart';

/// The Categories page as Rust last sent it: the top categories folded from
/// a snapshot and the appends after it, and the strip of categories the
/// live favorites are streaming. Lives for the app's lifetime.
class CategoriesHolder extends ChangeNotifier {
  StreamSubscription<RustSignalPack<CategoriesUpdate>>? _subscription;

  /// Null until the first snapshot arrives.
  CategoriesState? get state => _state;
  CategoriesState? _state;

  /// The categories the live favorites are streaming, in Rust's order.
  /// Empty until Rust sends some, and whenever none are live.
  List<FavoritesCategoryRow> get fromYourChannels => _fromYourChannels;
  List<FavoritesCategoryRow> _fromYourChannels = const [];

  /// Subscribes to Rust's updates, then asks for the current state.
  void connect() {
    _subscription = CategoriesUpdate.rustSignalStream.listen(
      (pack) => apply(pack.message),
    );
    const CategoriesReady().sendSignalToRust();
  }

  /// Folds one update in: a snapshot replaces the state, an append adds
  /// its categories after the loaded ones and replaces the footer, and a
  /// strip replaces the strip. Rust only appends to a loaded list.
  void apply(CategoriesUpdate update) {
    switch (update) {
      case CategoriesUpdateSnapshot(:final state):
        _state = state;
      case CategoriesUpdateAppended(:final categories, :final more):
        if (_state case CategoriesStateLoaded(categories: final loaded)) {
          _state = CategoriesStateLoaded(
            categories: [...loaded, ...categories],
            more: more,
          );
        }
      case CategoriesUpdateFromYourChannels(:final categories):
        _fromYourChannels = List.of(categories);
    }
    notifyListeners();
  }

  /// Starts the top categories over from the first page.
  void refresh() => const RefreshCategories().sendSignalToRust();

  /// Asks for the next page; Rust ignores it while one is loading or when
  /// there is none.
  void next() => const NextCategoriesPage().sendSignalToRust();

  @override
  void dispose() {
    unawaited(_subscription?.cancel());
    super.dispose();
  }
}
