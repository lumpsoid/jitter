import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:rinf/rinf.dart';

/// The player page as Rust last sent it. Lives for the app's lifetime.
///
/// There is no ready signal: the page's `Play` is answered with the state,
/// and nothing is shown before a page opens.
class PlaybackHolder extends ChangeNotifier {
  StreamSubscription<RustSignalPack<PlaybackUpdate>>? _subscription;

  PlaybackUpdate get state => _state;
  PlaybackUpdate _state = const PlaybackUpdateIdle();

  /// Subscribes to Rust's updates.
  void connect() {
    _subscription = PlaybackUpdate.rustSignalStream.listen(
      (pack) => apply(pack.message),
    );
  }

  /// Replaces the state.
  void apply(PlaybackUpdate update) {
    _state = update;
    notifyListeners();
  }

  @override
  void dispose() {
    unawaited(_subscription?.cancel());
    super.dispose();
  }
}
