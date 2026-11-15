import 'dart:async';

import 'process_load.dart';
import 'subjects.dart';
import 'targets.dart';

/// One run: the [subject]'s engine plays each of [streams] in turn, measures
/// it, stops it and waits [settle] before the next. The run writes its `RUN`,
/// `STREAM`, `RESULT` and `ERROR` lines through [log] and its short human
/// status through [status].
///
/// The device reset under study shows on the second stream, so a run is
/// several streams with a settle between them. Native device lines land in
/// that gap, in the same log.
///
/// [run] never throws. An error ends the stream it came from, after a try to
/// stop the engine; an error in [BenchSubject.prepare] ends the run.
class BenchRun {
  new({
    required this.subject,
    required this.streams,
    required this.warmup,
    required this.window,
    required this.log,
    required this.status,
    required this.visibility,
    LoadSampler? sampler,
    this.settle = const Duration(seconds: 3),
    this.delay = Future<void>.delayed,
  }) : _sampler = sampler ?? LoadSampler();

  final BenchSubject subject;

  /// Each target with its resolved stream URL.
  final List<(BenchTarget, String)> streams;
  final Duration warmup;
  final Duration window;
  final void Function(String line) log;
  final void Function(String text) status;

  /// The app lifecycle states seen since the last call, such as `resumed` or
  /// `resumed,inactive`.
  final String Function() visibility;
  final Duration settle;

  /// Waits for a span of time. A test replaces it so the run takes no real
  /// time.
  final Future<void> Function(Duration) delay;
  final LoadSampler _sampler;

  /// Runs the streams in order. Never throws.
  Future<void> run() async {
    final count = streams.length;
    log(
      'RUN engine=${subject.name} streams=$count'
      ' warmup_s=${warmup.inSeconds} window_s=${window.inSeconds}',
    );
    if (await _prepared()) {
      for (var i = 0; i < count; i++) {
        final (target, url) = streams[i];
        await _stream(i + 1, count, target.label, url);
      }
    }
    status('Done');
    log('RUN done');
  }

  /// Whether [BenchSubject.prepare] succeeded. A failure is logged here.
  Future<bool> _prepared() async {
    try {
      await subject.prepare();
      return true;
    } on Object catch (e) {
      log('ERROR prepare: $e');
      return false;
    }
  }

  /// One stream: open, warm up, measure, stop, then settle. A failure in the
  /// window is logged and the engine is still stopped.
  Future<void> _stream(int i, int n, String label, String url) async {
    final where = 'stream $i/$n';
    status('Stream $i/$n: $label, warming up');
    log('STREAM $i/$n $label open');
    var sampling = false;
    try {
      await subject.engine.open(url, start: Duration.zero);
      await delay(warmup);
      visibility();
      await subject.startWindow();
      await _sampler.start(window);
      sampling = true;
      final clock = Stopwatch()..start();
      status('Stream $i/$n: $label, measuring');
      await delay(window);
      clock.stop();
      final visible = visibility();
      final end = await subject.endWindow();
      final load = await _sampler.stop();
      sampling = false;
      log(
        'RESULT $i/$n $label engine=${subject.name}'
        ' seconds=${_seconds(clock)}'
        ' position_s=${subject.engine.position.inSeconds}'
        '$end${load.toFields()} visible=$visible',
      );
    } on Object catch (e) {
      log('ERROR $where: $e');
      // Ends the sampler's timer when the window had started.
      if (sampling) await _sampler.stop();
    }
    status('Stream $i/$n: $label, stopping');
    try {
      await subject.engine.stop();
      log('STREAM $i/$n stopped');
    } on Object catch (e) {
      log('ERROR $where stop: $e');
    }
    await delay(settle);
  }

  static String _seconds(Stopwatch clock) =>
      (clock.elapsedMicroseconds / Duration.microsecondsPerSecond)
          .toStringAsFixed(1);
}
