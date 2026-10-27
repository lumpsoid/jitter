import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:rinf/rinf.dart';

/// The search results as Rust last sent them: each list folded from its
/// snapshot and the appends after it. Lives for the app's lifetime.
class SearchHolder extends ChangeNotifier {
  StreamSubscription<RustSignalPack<SearchUpdate>>? _subscription;

  /// The channels matching the search; idle until something is searched.
  SearchChannelsState get channels => _channels;
  SearchChannelsState _channels = const SearchChannelsStateIdle();

  /// The categories matching the search; idle until something is searched.
  SearchCategoriesState get categories => _categories;
  SearchCategoriesState _categories = const SearchCategoriesStateIdle();

  /// Subscribes to Rust's updates. There is nothing to ask for until the
  /// user types, so no ready signal: `SearchTyped` is answered with both
  /// lists.
  void connect() {
    _subscription = SearchUpdate.rustSignalStream.listen(
      (pack) => apply(pack.message),
    );
  }

  /// Folds one update in: a snapshot replaces its list, an append adds its
  /// results after the loaded ones and replaces the footer. Rust only
  /// appends to a loaded list.
  void apply(SearchUpdate update) {
    switch (update) {
      case SearchUpdateChannels(:final state):
        _channels = state;
      case SearchUpdateChannelsAppended(:final channels, :final more):
        if (_channels case SearchChannelsStateLoaded(channels: final loaded)) {
          _channels = SearchChannelsStateLoaded(
            channels: [...loaded, ...channels],
            more: more,
          );
        }
      case SearchUpdateCategories(:final state):
        _categories = state;
      case SearchUpdateCategoriesAppended(:final categories, :final more):
        if (_categories case SearchCategoriesStateLoaded(
          categories: final loaded,
        )) {
          _categories = SearchCategoriesStateLoaded(
            categories: [...loaded, ...categories],
            more: more,
          );
        }
    }
    notifyListeners();
  }

  /// The field's text changed. Rust waits for typing to pause.
  void type(String text) => SearchTyped(text: text).sendSignalToRust();

  /// Starts list [list] over from its first page.
  void refresh(SearchList list) => RefreshSearch(list: list).sendSignalToRust();

  /// Asks for list [list]'s next page; Rust ignores it while one is
  /// loading or when there is none.
  void next(SearchList list) => NextSearchPage(list: list).sendSignalToRust();

  @override
  void dispose() {
    unawaited(_subscription?.cancel());
    super.dispose();
  }
}
