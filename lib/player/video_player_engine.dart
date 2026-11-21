import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:fvp/fvp.dart' show FvpCreateCancellation;
import 'package:jitter/logs/rust_log.dart';
import 'package:jitter/player/faded_volume.dart';
import 'package:jitter/player/live.dart';
import 'package:jitter/player/player_engine.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:video_player/video_player.dart';

/// [PlayerEngine] on `video_player`: fvp (libmdk) where fvp is registered, the
/// platform's own player (ExoPlayer on Android) elsewhere. Volumes are 0–100
/// here, as the user sees them, and 0–1 on the controller.
class VideoPlayerEngine implements PlayerEngine {
  /// [stepFrames] steps the paused picture by that many frames, back when
  /// negative. `video_player` has no frame step of its own, so with none,
  /// [stepFrame] does nothing.
  new({this.sendBuffering = _sendBuffering, this.stepFrames, this.log = _log});

  /// Sends the buffering reports; tests without Rust replace it.
  final void Function(PlayerBuffering) sendBuffering;

  /// Steps [VideoPlayerController] by `frames`; null where the platform has
  /// no frame step.
  final Future<void> Function(VideoPlayerController controller, int frames)?
  stepFrames;

  /// Logs a load that failed; tests without Rust replace it.
  final void Function(String) log;

  /// Counts each `open`, `stop` and `stopForExit`. A load whose number is no
  /// longer this one has been superseded, so it is dropped, never played.
  int _generation = 0;

  /// The load in flight, cancelled by the command that supersedes it.
  FvpCreateCancellation? _loading;

  /// Where the load starts: the Open's `start`, or a seek made while it
  /// loads, which replaces it.
  Duration _start = Duration.zero;

  final ValueNotifier<VideoPlayerController?> _controller =
      ValueNotifier<VideoPlayerController?>(null);

  final _volume = FadedVolume();

  /// The volume (0–100) the player plays at: under a fade, the faded one.
  double _playerVolume = 100;

  /// Each change of the playing controller, and each swap of it. Synchronous,
  /// so a stream reads the state as it is at the change, not after the
  /// swap that may follow it.
  final _changes = StreamController<void>.broadcast(sync: true);

  /// Each change of the player's volume, synchronous as [_changes] is.
  final _volumeChanges = StreamController<void>.broadcast(sync: true);

  /// The buffering last reported, so each change is reported once.
  bool _buffering = false;

  VideoPlayerValue? get _state => _controller.value?.value;

  /// Ends any fade first, so the new media plays at the user's volume.
  ///
  /// Returns once loading has started, not when the picture shows, so a slow
  /// load does not hold up the commands queued behind it. A failed load is
  /// logged. A load a later `open`, `stop` or `stopForExit` supersedes is
  /// dropped, never played.
  @override
  Future<void> open(String url, {required Duration start}) async {
    final generation = ++_generation;
    _cancelLoad();
    await _release();
    await _setPlayerVolume(_volume.end());
    if (generation != _generation) return;
    _start = start;
    final next = VideoPlayerController.networkUrl(Uri.parse(url));
    final cancellation = _loading = FvpCreateCancellation();
    unawaited(_load(next, cancellation, generation));
  }

  /// Stops, then ends a fade: silent until stopped, the user's volume
  /// after.
  @override
  Future<void> stop() async {
    _generation += 1;
    _cancelLoad();
    await _release();
    await _setPlayerVolume(_volume.end());
  }

  /// Stops what plays, if anything ever played, so the exit does not wait on
  /// the platform tearing down a live stream.
  @override
  Future<void> stopForExit() {
    _generation += 1;
    _cancelLoad();
    return _release();
  }

  @override
  Future<void> play() async {
    await _controller.value?.play();
  }

  @override
  Future<void> pause() async {
    await _controller.value?.pause();
  }

  @override
  Future<void> togglePlaying() async {
    if (isPlaying) {
      await pause();
    } else {
      await play();
    }
  }

  /// Whether it is playing now, not paused.
  @override
  bool get isPlaying => _state?.isPlaying ?? false;

  /// Whether it is playing now, then each change; an `open` passes
  /// through `false`.
  @override
  Stream<bool> get playing => _valuesFrom(() => isPlaying);

  @override
  Future<void> seek(Duration to) async {
    // A load still starting has no controller yet: its start moves instead.
    if (_controller.value == null && _loading != null) {
      _start = to;
      return;
    }
    await _controller.value?.seekTo(to);
  }

  /// Moves the position by [by], within the media: not before its start,
  /// nor past its end, which would end it. While no controller is published
  /// it does nothing: a load still starting has no position to step from.
  @override
  Future<void> seekBy(Duration by) async {
    if (_controller.value == null) return;
    final to = position + by;
    await seek(
      to < Duration.zero
          ? Duration.zero
          : to > duration
          ? duration
          : to,
    );
  }

  /// Steps one frame [forward] or back, while paused; playing, it does
  /// nothing, as the step would pause it.
  @override
  Future<void> stepFrame({required bool forward}) async {
    final controller = _controller.value;
    final step = stepFrames;
    if (isPlaying || controller == null || step == null) return;
    await step(controller, forward ? 1 : -1);
  }

  @override
  Duration get position => _state?.position ?? Duration.zero;

  /// Where it plays.
  @override
  Live<Duration> get positions => Live(
    now: () => position,
    changes: () => _changes.stream.map((_) => position).distinct(),
  );

  @override
  Duration get duration => _lengthOf(_state);

  /// How long the media is: zero until it has loaded, and zero for a live
  /// stream, whose length is unbounded.
  @override
  Live<Duration> get durations => Live(
    now: () => duration,
    changes: () => _changes.stream.map((_) => duration).distinct(),
  );

  /// Whether it waits for the media to load now, then each change.
  @override
  Stream<bool> get bufferings =>
      _valuesFrom(() => _state?.isBuffering ?? false);

  /// How far it has buffered.
  @override
  Live<Duration> get buffers => Live(
    now: () => _buffered,
    changes: () => _changes.stream.map((_) => _buffered).distinct(),
  );

  @override
  Future<void> setFade(double level) =>
      _setPlayerVolume(_volume.fade(level, current: _playerVolume));

  @override
  Future<void> toggleMute() =>
      _setPlayerVolume(_volume.toggleMute(current: _playerVolume));

  @override
  Future<void> nudgeVolume(double by) =>
      _setPlayerVolume(_volume.nudge(by, current: _playerVolume));

  /// The user's volume now, then each change; under a fade, the one it fades
  /// from.
  @override
  Live<double> get volumes => Live(
    now: () => _volume.user(current: _playerVolume),
    changes: () => _volumeChanges.stream
        .map((_) => _volume.user(current: _playerVolume))
        .distinct(),
  );

  /// Sets the user's volume; mid-fade, under the fade.
  @override
  Future<void> setVolume(double to) =>
      _setPlayerVolume(_volume.set(to, current: _playerVolume));

  /// The picture of the playing controller, under [zoom], letterboxed in
  /// black; black alone while none plays.
  @override
  Widget surface(Widget Function(Widget picture) zoom) =>
      ValueListenableBuilder<VideoPlayerController?>(
        valueListenable: _controller,
        builder: (_, controller, _) => controller == null
            ? const ColoredBox(color: Color(0xFF000000))
            : ColoredBox(
                color: const Color(0xFF000000),
                child: Center(
                  child: AspectRatio(
                    aspectRatio: controller.value.aspectRatio,
                    child: zoom(VideoPlayer(controller)),
                  ),
                ),
              ),
      );

  /// How far it has buffered: the end of the last range, zero if none.
  Duration get _buffered {
    final ranges = _state?.buffered ?? const <DurationRange>[];
    return ranges.isEmpty ? Duration.zero : ranges.last.end;
  }

  /// Sets the player's volume to [to] (0–100) when a change is due; null
  /// means none is.
  Future<void> _setPlayerVolume(double? to) async {
    if (to == null) return;
    _playerVolume = to;
    _volumeChanges.add(null);
    await _controller.value?.setVolume(to / 100);
  }

  /// Loads [next], publishes it, then starts it. A load a later command
  /// supersedes before it is published is dropped. Once published, the
  /// controller belongs to [_release], so a superseded load leaves it there.
  /// Only a current load's failure is logged. A controller that fails before
  /// it is published is disposed without being awaited: a create that threw
  /// never completes, so its dispose never does.
  Future<void> _load(
    VideoPlayerController next,
    FvpCreateCancellation cancellation,
    int generation,
  ) async {
    var published = false;
    try {
      await cancellation.run(next.initialize);
      if (generation != _generation) {
        next.dispose().ignore();
        return;
      }
      _loading = null;
      _publish(next);
      published = true;
      if (_start > Duration.zero) {
        await next.seekTo(_start);
      }
      if (generation != _generation) return;
      // Read after the waits, so a fade set meanwhile counts.
      await next.setVolume(_playerVolume / 100);
      if (generation != _generation) return;
      await next.play();
    } on Object catch (error) {
      if (!published) next.dispose().ignore();
      if (generation == _generation) {
        _loading = null;
        log('$error');
      }
    }
  }

  /// Cancels the load in flight, if any, so its wait ends at once.
  void _cancelLoad() {
    _loading?.cancel();
    _loading = null;
  }

  /// Takes [next] for the playing controller, and tells the streams.
  void _publish(VideoPlayerController next) {
    _buffering = false;
    next.addListener(_notice);
    _controller.value = next;
    _changes.add(null);
  }

  /// Stops the playing controller, if any, and disposes it.
  Future<void> _release() async {
    final previous = _controller.value;
    if (previous != null && _buffering) {
      // A wait the stream ends is reported as over, as the stream's own stop
      // would, so the page's stall watch does not wait for it.
      _buffering = false;
      sendBuffering(
        PlayerBuffering(
          buffering: false,
          positionMs: previous.value.position.inMilliseconds,
          bufferMs: _buffered.inMilliseconds,
        ),
      );
    }
    _controller.value = null;
    previous?.removeListener(_notice);
    _changes.add(null);
    await previous?.pause();
    await previous?.dispose();
  }

  /// Each change of the playing controller: a buffering change is reported
  /// once, then the streams are told.
  void _notice() {
    final state = _state;
    if (state != null && state.isBuffering != _buffering) {
      _buffering = state.isBuffering;
      sendBuffering(
        PlayerBuffering(
          buffering: _buffering,
          positionMs: state.position.inMilliseconds,
          bufferMs: _buffered.inMilliseconds,
        ),
      );
    }
    _changes.add(null);
  }

  /// [now] first, then each value it takes after a change, skipping repeats.
  ///
  /// Not `async*` with an `await for`: cancelling that waits for the next
  /// change, which may never come.
  Stream<T> _valuesFrom<T>(T Function() now) => Stream<T>.multi((out) {
    var last = now();
    out.add(last);
    final changes = _changes.stream.listen((_) {
      final next = now();
      if (next != last) {
        last = next;
        out.add(next);
      }
    });
    out.onCancel = changes.cancel;
  });
}

/// Longer than any Twitch VOD (48 h). mdk reports a live stream's length as
/// unbounded, the largest duration it can hold, which is no length at all.
const _unboundedAbove = Duration(days: 1000);

/// The media's length from [state]: zero with no state, and zero when the
/// length is unbounded.
Duration _lengthOf(VideoPlayerValue? state) {
  final length = state?.duration ?? Duration.zero;
  return length > _unboundedAbove ? Duration.zero : length;
}

void _sendBuffering(PlayerBuffering signal) => signal.sendSignalToRust();

void _log(String message) => logToRust(DartLogLevel.error, 'player', message);
