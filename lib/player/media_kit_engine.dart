import 'dart:async';
import 'dart:io';

import 'package:flutter/services.dart';
import 'package:jitter/logs/rust_log.dart';
import 'package:jitter/player/faded_volume.dart';
import 'package:jitter/player/live.dart';
import 'package:jitter/player/mpv_probes.dart';
import 'package:jitter/player/player_engine.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:media_kit/media_kit.dart';
import 'package:media_kit_video/media_kit_video.dart';
import 'package:path_provider/path_provider.dart';

/// The mpv `hwdec` to decode with under the [environment]; `null` keeps
/// media_kit's `auto`. `JITTER_HWDEC`, when set and not empty, is used
/// as-is, to try another decoder (`d3d11va-copy`, or `no` for software)
/// without a rebuild, and wins over the gamescope case.
///
/// Under gamescope (SteamOS game mode), `auto` tries Vulkan decoding, and
/// gamescope's WSI layer aborts the process when FFmpeg 9 creates its
/// Vulkan device: the layer's vkroots fetches the device's queues with
/// `vkGetDeviceQueue`, which gives none for queues FFmpeg creates
/// internally synchronized (misyltoad/vkroots#17). VA-API decodes the
/// same streams there; `vaapi-copy` keeps it when the video is drawn
/// without EGL.
///
/// Workaround: remove the gamescope case and its test once SteamOS ships a
/// gamescope whose vkroots has that fix (ValveSoftware/gamescope#2261). How
/// to check, and the full trail: `docs/gamescope-vulkan-hwdec.md`. The
/// `JITTER_HWDEC` override stays.
String? hwdecFor(Map<String, String> environment) {
  if (environment['JITTER_HWDEC'] case final String hwdec
      when hwdec.isNotEmpty) {
    return hwdec;
  }
  return environment.containsKey('GAMESCOPE_WAYLAND_DISPLAY')
      ? 'vaapi,vaapi-copy'
      : null;
}

/// The log's level for an mpv message at [level] (`fatal`, `error`,
/// `warn`, `info`, `v`). Rust keeps only the video's lines of the verbose
/// ones.
DartLogLevel _logLevel(String level) => switch (level) {
  'fatal' || 'error' => DartLogLevel.error,
  'warn' => DartLogLevel.warning,
  _ => DartLogLevel.verbose,
};

/// [PlayerEngine] on `media_kit` (libmpv). One for the app's lifetime; the
/// player page shows [controller]. The player is created on first use, so
/// building this needs no libmpv.
class MediaKitEngine implements PlayerEngine {
  Player get _player => _created ??= _create();
  Player? _created;
  MpvProbes? _probes;

  /// Points mpv's on-disk cache, which media_kit turns on, at the app's
  /// cache directory: with media_kit's setup, mpv's own default gives it no
  /// folder, so it fails and caches in memory alone. Done before the first
  /// [open].
  Future<void>? _cacheDir;

  /// The player, reporting to this run's log: mpv's warnings and errors,
  /// what its decoder and renderer do, each buffering wait with where it
  /// began and what was buffered, the probes: mpv's versions and GL
  /// context, and how it plays each stream, and, on Windows, what
  /// media_kit's video output does.
  Player _create() {
    final player = Player(
      configuration: const PlayerConfiguration(
        // What the OS volume mixer shows for our audio stream.
        title: 'Jitter',
        // Verbose, for how mpv sets up and tears down the video.
        logLevel: MPVLogLevel.v,
      ),
    );
    player.stream.log.listen(
      (log) => logToRust(_logLevel(log.level), 'mpv/${log.prefix}', log.text),
    );
    player.stream.buffering.listen(
      (buffering) => PlayerBuffering(
        buffering: buffering,
        positionMs: player.state.position.inMilliseconds,
        bufferMs: player.state.buffer.inMilliseconds,
      ).sendSignalToRust(),
    );
    _videoOutputLog.setMethodCallHandler(_logVideoOutput);
    if (player.platform case final NativePlayer mpv) {
      final probes = _probes = MpvProbes(mpv);
      unawaited(probes.versions());
      unawaited(probes.watchOutput());
      _cacheDir = _setCacheDir(mpv);
      player.stream.videoParams.listen(probes.videoParams);
    }
    return player;
  }

  /// What media_kit's Windows video output reports: its textures, and the
  /// first time a D3D 11 device it draws with stops working, with Windows'
  /// reason. Its patch: `third_party/media_kit_video/PATCHED.md`.
  static const _videoOutputLog = MethodChannel('jitter/media_kit_video/log');

  static Future<void> _logVideoOutput(MethodCall call) async {
    if (call.arguments case {
      'level': final String level,
      'message': final String message,
    }) {
      logToRust(
        level == 'error' ? DartLogLevel.error : DartLogLevel.info,
        'media_kit/video',
        message,
      );
    }
  }

  /// What the player page's `Video` renders.
  late final VideoController controller = VideoController(
    _player,
    configuration: VideoControllerConfiguration(
      hwdec: hwdecFor(Platform.environment),
    ),
  );

  final _volume = FadedVolume();

  /// Ends a fade first, so the new media plays at the user's volume.
  @override
  Future<void> open(String url, {required Duration start}) async {
    await _setVolume(_volume.end());
    await _probes?.ended();
    await _cacheDir;
    await _player.open(Media(url, start: start));
  }

  /// Without a cache directory, mpv keeps its default.
  static Future<void> _setCacheDir(NativePlayer mpv) async {
    try {
      final dir = await getApplicationCacheDirectory();
      await mpv.setProperty('demuxer-cache-dir', dir.path);
    } on Object catch (error) {
      logToRust(DartLogLevel.warning, 'player', 'No cache directory: $error');
    }
  }

  /// Stops, then ends a fade: silent until stopped, the user's volume
  /// after.
  @override
  Future<void> stop() async {
    await _probes?.ended();
    await _player.stop();
    await _setVolume(_volume.end());
  }

  /// Stops what plays, if a player was ever created, so the exit does not
  /// wait on libmpv tearing down a live stream and its video output.
  Future<void> stopForExit() async {
    await _probes?.ended();
    await _created?.stop();
  }

  @override
  Future<void> setFade(double level) =>
      _setVolume(_volume.fade(level, current: _player.state.volume));

  Future<void> togglePlaying() => _player.playOrPause();

  Future<void> play() => _player.play();

  Future<void> pause() => _player.pause();

  /// Whether it is playing now, then each change; an `open` passes
  /// through `false`.
  Stream<bool> get playing async* {
    yield _player.state.playing;
    yield* _player.stream.playing;
  }

  @override
  Future<void> seek(Duration to) => _player.seek(to);

  /// Moves the position by [by], within the media: not before its start,
  /// nor past its end, which would end it.
  Future<void> seekBy(Duration by) {
    final to = position + by;
    return _player.seek(
      to < Duration.zero
          ? Duration.zero
          : to > duration
          ? duration
          : to,
    );
  }

  /// Steps one frame [forward] or back, while paused; playing, it does
  /// nothing, as mpv's step would pause it.
  Future<void> stepFrame({required bool forward}) async {
    if (_player.state.playing) return;
    if (_player.platform case final NativePlayer mpv) {
      await mpv.command([if (forward) 'frame-step' else 'frame-back-step']);
    }
  }

  /// Where it plays.
  Live<Duration> get positions => Live(
    now: () => _player.state.position,
    changes: () => _player.stream.position,
  );

  /// How long the media is: zero until it has loaded.
  Live<Duration> get durations => Live(
    now: () => _player.state.duration,
    changes: () => _player.stream.duration,
  );

  /// Whether it waits for the media to load now, then each change.
  Stream<bool> get bufferings async* {
    yield _player.state.buffering;
    yield* _player.stream.buffering;
  }

  /// How far it has buffered.
  Live<Duration> get buffers => Live(
    now: () => _player.state.buffer,
    changes: () => _player.stream.buffer,
  );

  /// Mutes, or unmutes back to the volume it was muted at; mid-fade, under
  /// the fade.
  Future<void> toggleMute() =>
      _player.setVolume(_volume.toggleMute(current: _player.state.volume));

  /// Moves the volume by [by] (of 0–100); mid-fade, under the fade.
  Future<void> nudgeVolume(double by) =>
      _player.setVolume(_volume.nudge(by, current: _player.state.volume));

  Future<void> _setVolume(double? volume) async {
    if (volume != null) await _player.setVolume(volume);
  }

  /// Whether it is playing now, not paused.
  bool get isPlaying => _player.state.playing;

  @override
  Duration get position => _player.state.position;

  @override
  Duration get duration => _player.state.duration;
}
