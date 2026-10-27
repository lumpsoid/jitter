import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:rinf/rinf.dart';

/// The open channel as Rust last sent it: its header, and its VODs folded
/// from a snapshot and the appends after it. Lives for the app's lifetime;
/// each channel page [open]s its channel in it.
class ChannelHolder extends ChangeNotifier {
  StreamSubscription<RustSignalPack<ChannelUpdate>>? _headers;
  StreamSubscription<RustSignalPack<VodsUpdate>>? _vodUpdates;

  /// Null from [open] until Rust answers it.
  ChannelUpdate? get header => _header;
  ChannelUpdate? _header;

  /// Null from [open] until Rust answers it.
  VodsState? get vods => _vods;
  VodsState? _vods;

  /// Subscribes to Rust's updates. There is nothing to ask for until a
  /// page [open]s a channel, so no ready signal: `OpenChannel` is it.
  void connect() {
    _headers = ChannelUpdate.rustSignalStream.listen(
      (pack) => applyHeader(pack.message),
    );
    _vodUpdates = VodsUpdate.rustSignalStream.listen(
      (pack) => applyVods(pack.message),
    );
  }

  /// Forgets the channel shown before, then asks Rust for [login]'s header
  /// and its first kind of VODs.
  void open(String login) {
    _header = null;
    _vods = null;
    notifyListeners();
    OpenChannel(login: login).sendSignalToRust();
  }

  /// Replaces the header.
  void applyHeader(ChannelUpdate update) {
    _header = update;
    notifyListeners();
  }

  /// Folds one VODs update in: a snapshot replaces the list, an append adds
  /// its VODs after the loaded ones and replaces the footer. Rust only
  /// appends to a loaded list.
  void applyVods(VodsUpdate update) {
    _vods = switch (update) {
      VodsUpdateSnapshot(:final state) => state,
      VodsUpdateAppended(:final vods, :final more) => switch (_vods) {
        VodsStateLoaded(vods: final loaded) => VodsStateLoaded(
          vods: [...loaded, ...vods],
          more: more,
        ),
        final current => current,
      },
      _ => _vods,
    };
    notifyListeners();
  }

  /// Lists the open channel's VODs of [kind] instead.
  void selectKind(VodKind kind) => SelectVodKind(kind: kind).sendSignalToRust();

  /// Looks the open channel up again and starts its VODs over.
  void refresh() => const RefreshChannel().sendSignalToRust();

  /// Asks for the next page of VODs; Rust ignores it while one is loading
  /// or when there is none.
  void next() => const VodsNextPage().sendSignalToRust();

  @override
  void dispose() {
    unawaited(_headers?.cancel());
    unawaited(_vodUpdates?.cancel());
    super.dispose();
  }
}
