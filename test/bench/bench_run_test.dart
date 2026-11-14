import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/player_engine.dart';

import '../../bench/bench_run.dart';
import '../../bench/process_load.dart';
import '../../bench/subjects.dart';
import '../../bench/targets.dart';

/// Records the URLs it is asked to open and counts its stops. The open of
/// [failOn] throws after it is recorded.
class _FakeEngine implements PlayerEngine {
  final List<String> opened = [];
  int stops = 0;
  String? failOn;

  @override
  Future<void> open(String url, {required Duration start}) async {
    opened.add(url);
    if (url == failOn) throw StateError('no such stream: $url');
  }

  @override
  Future<void> stop() async {
    stops++;
  }

  @override
  Future<void> setFade(double level) async {}

  @override
  Future<void> seek(Duration to) async {}

  @override
  Duration get position => const Duration(seconds: 42);

  @override
  Duration get duration => Duration.zero;
}

class _FakeSubject implements BenchSubject {
  new(this.engine, {this.failPrepare = false});

  final bool failPrepare;

  @override
  final PlayerEngine engine;

  @override
  String get name => 'fake';

  @override
  Widget get picture => const SizedBox();

  @override
  Future<void> prepare() async {
    if (failPrepare) throw StateError('no libmpv handle');
  }

  @override
  Future<void> startWindow() async {}

  @override
  Future<String> endWindow() async => ' x=1';
}

const List<(BenchTarget, String)> _streams = [
  (LiveTarget('a'), 'https://a.example/live.m3u8'),
  (VodTarget('7'), 'https://vod.example/7.m3u8'),
];

Future<List<String>> _run({
  required BenchSubject subject,
  List<(BenchTarget, String)> streams = _streams,
}) async {
  final lines = <String>[];
  await BenchRun(
    subject: subject,
    streams: streams,
    warmup: const Duration(seconds: 10),
    window: const Duration(seconds: 30),
    log: lines.add,
    status: (_) {},
    visibility: () => 'resumed',
    sampler: LoadSampler(),
    delay: (_) async {},
  ).run();
  return lines;
}

void main() {
  group('BenchRun', () {
    test(
      'plays two streams in order, logging each open, result and stop',
      () async {
        final engine = _FakeEngine();
        final lines = await _run(subject: _FakeSubject(engine));

        expect(lines, hasLength(8));
        const prefixes = [
          'RUN engine=fake streams=2 warmup_s=10 window_s=30',
          'STREAM 1/2 live:a open',
          'RESULT 1/2 live:a engine=fake seconds=',
          'STREAM 1/2 stopped',
          'STREAM 2/2 vod:7 open',
          'RESULT 2/2',
          'STREAM 2/2 stopped',
          'RUN done',
        ];
        for (var i = 0; i < prefixes.length; i++) {
          expect(lines[i], startsWith(prefixes[i]));
        }
        for (final result in [lines[2], lines[5]]) {
          expect(result, contains(' x=1'));
          expect(result, contains(' cpu_pct='));
          expect(result, endsWith(' visible=resumed'));
        }
        expect(engine.opened, [_streams[0].$2, _streams[1].$2]);
        expect(engine.stops, 2);
      },
    );

    test(
      'an open that fails logs an error, stops the engine and goes on',
      () async {
        final engine = _FakeEngine()..failOn = _streams[0].$2;
        final lines = await _run(subject: _FakeSubject(engine));

        expect(lines, hasLength(8));
        expect(lines[1], 'STREAM 1/2 live:a open');
        expect(lines[2], startsWith('ERROR stream 1/2: '));
        expect(lines[3], 'STREAM 1/2 stopped');
        expect(lines[4], 'STREAM 2/2 vod:7 open');
        expect(lines[5], startsWith('RESULT 2/2'));
        expect(lines[6], 'STREAM 2/2 stopped');
        expect(lines[7], 'RUN done');
        expect(engine.opened, [_streams[0].$2, _streams[1].$2]);
        expect(engine.stops, 2);
      },
    );

    test('a prepare error ends the run before any stream opens', () async {
      final engine = _FakeEngine();
      final lines = await _run(
        subject: _FakeSubject(engine, failPrepare: true),
      );

      expect(lines, hasLength(3));
      expect(lines[0], 'RUN engine=fake streams=2 warmup_s=10 window_s=30');
      expect(lines[1], startsWith('ERROR prepare: '));
      expect(lines[2], 'RUN done');
      expect(engine.opened, isEmpty);
    });
  });
}
