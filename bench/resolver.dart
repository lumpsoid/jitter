import 'dart:async';

import 'package:jitter/src/bindings/bindings.dart';
import 'package:path_provider/path_provider.dart';

import 'targets.dart';

/// Boots the hub, as the app does at start, and waits for its answer. The
/// hub answers a repeated `Boot` too, so this may be called again.
///
/// Throws a [StateError] with the hub's message when the store cannot open,
/// or when no answer comes within [timeout].
Future<void> bootHub({Duration timeout = const Duration(seconds: 30)}) async {
  final ready = Completer<void>();
  // The signal streams are broadcast and do not replay, so both listeners
  // exist before `Boot` is sent.
  final answered = Booted.rustSignalStream.listen((_) {
    if (!ready.isCompleted) ready.complete();
  });
  final failed = BootFailed.rustSignalStream.listen((pack) {
    if (!ready.isCompleted) {
      ready.completeError(StateError(pack.message.message));
    }
  });
  try {
    final dir = await getApplicationDocumentsDirectory();
    Boot(dir: dir.path).sendSignalToRust();
    await ready.future.timeout(
      timeout,
      onTimeout: () => throw StateError(
        'the hub did not boot within ${timeout.inSeconds} s',
      ),
    );
  } finally {
    await answered.cancel();
    await failed.cancel();
  }
}

/// The stream URL for [target], which must come after [bootHub].
///
/// A [UrlTarget] is returned as typed and the hub is not touched. A live or
/// VOD target is played through the hub, and its URL is the first one the
/// hub opens after `Play`. Throws a [StateError] with the hub's message when
/// the hub fails the target, or with `'<label>: no stream URL after <n> s'`
/// when none comes within [timeout].
///
/// Only `Play` is sent. The bench sends no `LeavePlayer` or
/// `PlayerPosition`, so no watch history is written.
Future<String> resolveUrl(
  BenchTarget target, {
  Duration timeout = const Duration(seconds: 45),
}) {
  return switch (target) {
    UrlTarget(:final url) => Future.value(url),
    LiveTarget(:final login) => _resolveThroughHub(
      PlaybackTargetLive(login: login),
      target.label,
      timeout,
    ),
    VodTarget(:final id) => _resolveThroughHub(
      PlaybackTargetVod(id: id),
      target.label,
      timeout,
    ),
  };
}

Future<String> _resolveThroughHub(
  PlaybackTarget target,
  String label,
  Duration timeout,
) async {
  final url = Completer<String>();
  // The stall watchdog can send a later Open for the same stream, so only
  // the first Open after `Play` is this target's URL.
  final opened = PlayerCommand.rustSignalStream.listen((pack) {
    final command = pack.message;
    if (command is PlayerCommandOpen && !url.isCompleted) {
      url.complete(command.url);
    }
  });
  final failed = PlaybackUpdate.rustSignalStream.listen((pack) {
    final update = pack.message;
    if (update is PlaybackUpdateFailed && !url.isCompleted) {
      url.completeError(StateError(update.message));
    }
  });
  try {
    Play(target: target).sendSignalToRust();
    return await url.future.timeout(
      timeout,
      onTimeout: () => throw StateError(
        '$label: no stream URL after ${timeout.inSeconds} s',
      ),
    );
  } finally {
    await opened.cancel();
    await failed.cancel();
  }
}
