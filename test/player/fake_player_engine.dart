import 'package:jitter/player/live.dart';
import 'package:jitter/player/player_engine.dart';

/// A player that plays nothing and reports nothing, for tests that build
/// the app without libmpv.
class FakePlayerEngine implements PlayerEngine {
  @override
  Future<void> open(String url, {required Duration start}) async {}

  @override
  Future<void> stop() async {}

  @override
  Future<void> stopForExit() async {}

  @override
  Future<void> play() async {}

  @override
  Future<void> pause() async {}

  @override
  Future<void> togglePlaying() async {}

  @override
  bool get isPlaying => false;

  @override
  Stream<bool> get playing => const Stream.empty();

  @override
  Future<void> seek(Duration to) async {}

  @override
  Future<void> seekBy(Duration by) async {}

  @override
  Future<void> stepFrame({required bool forward}) async {}

  @override
  Duration get position => Duration.zero;

  @override
  Live<Duration> get positions =>
      Live(now: () => Duration.zero, changes: Stream.empty);

  @override
  Duration get duration => Duration.zero;

  @override
  Live<Duration> get durations =>
      Live(now: () => Duration.zero, changes: Stream.empty);

  @override
  Stream<bool> get bufferings => const Stream.empty();

  @override
  Live<Duration> get buffers =>
      Live(now: () => Duration.zero, changes: Stream.empty);

  @override
  Future<void> setFade(double level) async {}

  @override
  Future<void> toggleMute() async {}

  @override
  Future<void> nudgeVolume(double by) async {}

  @override
  Live<double> get volumes => Live(now: () => 100, changes: Stream.empty);

  @override
  Future<void> setVolume(double to) async {}
}
