import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:rinf/rinf.dart';

/// The downloads as Rust last sent them. Rust owns them and runs the
/// downloads; this holder mirrors the list for the downloads page and the
/// VOD action. Lives for the app's lifetime.
class DownloadsHolder extends ChangeNotifier {
  new({
    this.sendChoose = _sendChoose,
    this.sendStart = _sendStart,
    this.sendCancel = _sendCancel,
    this.sendRemove = _sendRemove,
  });

  /// Send the asks; tests without Rust replace them.
  final void Function(ChooseDownload) sendChoose;
  final void Function(StartDownload) sendStart;
  final void Function(CancelDownload) sendCancel;
  final void Function(RemoveDownload) sendRemove;

  StreamSubscription<RustSignalPack<DownloadsUpdate>>? _subscription;

  /// Rust's one-shot answers, `Choices` and `Failed`. Not state.
  final _notices = StreamController<DownloadsUpdate>.broadcast(sync: true);

  /// One message per failure Rust reports, for a snackbar each. Not state.
  Stream<String> get failures => _notices.stream
      .where((notice) => notice is DownloadsUpdateFailed)
      .map((notice) => (notice as DownloadsUpdateFailed).message);

  /// Empty until the first snapshot arrives.
  List<DownloadItem> get downloads => _downloads;
  List<DownloadItem> _downloads = const [];

  /// Subscribes to Rust's updates, then asks for the current downloads.
  void connect() {
    _subscription = DownloadsUpdate.rustSignalStream.listen(
      (pack) => apply(pack.message),
    );
    const DownloadsReady().sendSignalToRust();
  }

  /// Asks Rust for the quality menu of [videoId]. Completes with its
  /// `Choices`, or with its `Failed` when they cannot be loaded; answers
  /// for other VODs are skipped.
  Future<DownloadsUpdate> choose(String videoId) {
    // Listen before asking: a broadcast stream does not replay an answer.
    final answer = _notices.stream.firstWhere(
      (notice) => switch (notice) {
        DownloadsUpdateChoices(videoId: final id) ||
        DownloadsUpdateFailed(videoId: final id) => id == videoId,
        _ => false,
      },
    );
    sendChoose(ChooseDownload(videoId: videoId));
    return answer;
  }

  void start({
    required String videoId,
    required String quality,
    required String dir,
  }) => sendStart(StartDownload(videoId: videoId, quality: quality, dir: dir));

  void cancel(Uint64 key) => sendCancel(CancelDownload(key: key));

  void remove(Uint64 key) => sendRemove(RemoveDownload(key: key));

  /// Folds one update in: a snapshot replaces the downloads; choices and
  /// failures leave them alone and go to [choose] and [failures].
  void apply(DownloadsUpdate update) {
    switch (update) {
      case DownloadsUpdateSnapshot(:final downloads):
        _downloads = downloads;
        notifyListeners();
      case DownloadsUpdateChoices() || DownloadsUpdateFailed():
        _notices.add(update);
    }
  }

  @override
  void dispose() {
    unawaited(_subscription?.cancel());
    unawaited(_notices.close());
    super.dispose();
  }
}

void _sendChoose(ChooseDownload signal) => signal.sendSignalToRust();

void _sendStart(StartDownload signal) => signal.sendSignalToRust();

void _sendCancel(CancelDownload signal) => signal.sendSignalToRust();

void _sendRemove(RemoveDownload signal) => signal.sendSignalToRust();
