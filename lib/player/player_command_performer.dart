import 'dart:async';

import 'package:jitter/logs/rust_log.dart';
import 'package:jitter/player/player_engine.dart';
import 'package:jitter/src/bindings/bindings.dart';

/// Carries out each [PlayerCommand] Rust sends, one engine call per
/// command, in arrival order. Keeps nothing.
class PlayerCommandPerformer {
  new(this._engine, {this.log = _log});

  final PlayerEngine _engine;

  /// Logs a failed call; tests without Rust replace it.
  final void Function(String) log;

  Future<void> _last = Future.value();

  /// Performs every command [commands] delivers; production passes
  /// `PlayerCommand.rustSignalStream`'s messages.
  void listen(Stream<PlayerCommand> commands) {
    commands.listen((command) => unawaited(perform(command)));
  }

  /// Queues [command] behind the ones before it. A failed call is logged
  /// and does not hold up the next.
  Future<void> perform(PlayerCommand command) {
    return _last = _last
        .then((_) => _call(command))
        .catchError((Object error) => log('$error'));
  }

  Future<void> _call(PlayerCommand command) => switch (command) {
    PlayerCommandOpen(:final url, :final startS) => _engine.open(
      url,
      start: Duration(seconds: startS),
    ),
    PlayerCommandStop() => _engine.stop(),
    PlayerCommandSetFade(:final level) => _engine.setFade(level),
    PlayerCommandSeek(:final positionS) => _engine.seek(
      Duration(seconds: positionS),
    ),
    _ => Future.value(),
  };
}

void _log(String message) =>
    logToRust(DartLogLevel.error, 'player command', message);
