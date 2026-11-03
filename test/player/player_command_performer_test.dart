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
  Future<void> setFade(double level) async => calls.add('fade $level');

  @override
  Future<void> seek(Duration to) async => calls.add('seek ${to.inSeconds}');

  @override
  Duration get position => Duration.zero;

  @override
  Duration get duration => Duration.zero;
}

void main() {
  late _FakeEngine engine;
  late PlayerCommandPerformer performer;
  late List<String> logged;

  setUp(() {
    engine = _FakeEngine();
    logged = [];
    performer = PlayerCommandPerformer(engine, log: logged.add);
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

  test('a fade is one engine call, in order with the rest', () async {
    await performer.perform(const PlayerCommandSetFade(level: 0.5));
    await performer.perform(const PlayerCommandSetFade(level: 0.25));
    await performer.perform(const PlayerCommandStop());

    expect(engine.calls, ['fade 0.5', 'fade 0.25', 'stop']);
  });

  test('a seek goes to the position, in seconds', () async {
    await performer.perform(const PlayerCommandSeek(positionS: 5025));

    expect(engine.calls, ['seek 5025']);
  });

  test('a failed call is logged and does not hold up the next', () async {
    engine.failOpen = true;
    unawaited(performer.perform(const PlayerCommandOpen(url: 'a', startS: 0)));
    await performer.perform(const PlayerCommandStop());

    expect(engine.calls, ['stop']);
    expect(logged, ['Bad state: no libmpv']);
  });
}
