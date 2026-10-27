import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/player_command_performer.dart';
import 'package:jitter/player/player_engine.dart';
import 'package:jitter/src/bindings/bindings.dart';

/// Records each engine call. Opening takes a moment, so a later command
/// that overtook it would show up out of order.
class _FakeEngine implements PlayerEngine {
  final calls = <String>[];
  bool failOpen = false;

  @override
  Future<void> open(String url, {required Duration start}) async {
    await Future<void>.delayed(const Duration(milliseconds: 5));
    if (failOpen) throw StateError('no libmpv');
    calls.add('open $url at ${start.inSeconds}');
  }

  @override
  Future<void> stop() async => calls.add('stop');

  @override
  Duration get position => Duration.zero;

  @override
  Duration get duration => Duration.zero;
}

void main() {
  late _FakeEngine engine;
  late PlayerCommandPerformer performer;

  setUp(() {
    engine = _FakeEngine();
    performer = PlayerCommandPerformer(engine);
  });

  test('each command is one engine call', () async {
    await performer.perform(
      const PlayerCommandOpen(url: 'https://x.test/a.m3u8', startS: 600),
    );
    await performer.perform(const PlayerCommandStop());

    expect(engine.calls, ['open https://x.test/a.m3u8 at 600', 'stop']);
  });

  test('commands from the stream run in arrival order', () async {
    final commands = StreamController<PlayerCommand>();
    performer.listen(commands.stream);

    commands
      ..add(const PlayerCommandOpen(url: 'a', startS: 0))
      ..add(const PlayerCommandStop());
    await commands.close();
    await performer.perform(const PlayerCommandOpen(url: 'b', startS: 5));

    expect(engine.calls, ['open a at 0', 'stop', 'open b at 5']);
  });

  test('a failed call does not hold up the next', () async {
    engine.failOpen = true;
    unawaited(performer.perform(const PlayerCommandOpen(url: 'a', startS: 0)));
    await performer.perform(const PlayerCommandStop());

    expect(engine.calls, ['stop']);
  });
}
