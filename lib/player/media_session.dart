import 'dart:async';

import 'package:audio_service/audio_service.dart';
import 'package:jitter/player/media_kit_engine.dart';
import 'package:jitter/player/playback_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';

/// How long the player must report "not playing" before the system hears
/// it. An `open` passes through "not playing", and Rust opens the
/// audio-only quality just as the app goes to the background: on Android
/// that dip would drop the foreground service, which may not start again
/// from the background (Android 12+).
const _settle = Duration(seconds: 1);

/// The playing stream as the system shows it: Android's media
/// notification, whose foreground service keeps the app alive in the
/// background, the lock screen and headset buttons. Their play and pause
/// go to the player, as the page's own button does. Folds what Rust sent
/// and what the player reports; decides nothing.
class MediaSession extends BaseAudioHandler {
  new(this._playback, this._player);

  final PlaybackHolder _playback;
  final MediaKitEngine _player;
  bool _playing = false;
  Timer? _settling;

  /// Follows the player page's state and the player from now on.
  void connect() {
    _playback.addListener(_pageChanged);
    _player.playing.listen(_playerChanged);
    _pageChanged();
  }

  void _pageChanged() {
    mediaItem.add(mediaItemOf(_playback.state));
    _publish();
  }

  void _playerChanged(bool playing) {
    _settling?.cancel();
    if (playing) {
      _playing = true;
      _publish();
    } else {
      _settling = Timer(_settle, () {
        _playing = false;
        _publish();
      });
    }
  }

  void _publish() =>
      playbackState.add(sessionStateOf(_playback.state, playing: _playing));

  @override
  Future<void> play() => _player.play();

  @override
  Future<void> pause() => _player.pause();

  /// The notification offers no stop, but the system may still ask (a
  /// headset's stop button). Pausing leaves the page as Rust knows it.
  @override
  Future<void> stop() => _player.pause();
}

/// What the system shows for the player page's [state]: the channel, with
/// the game under it. `null` outside a playing page.
MediaItem? mediaItemOf(PlaybackUpdate state) => switch (state) {
  PlaybackUpdateReady(:final details, :final live) => MediaItem(
    id: details.channel?.login ?? 'jitter',
    title: details.channel?.displayName ?? 'Jitter',
    artist: details.game,
    artUri: switch (details.channel?.avatarUrl) {
      null => null,
      final url => Uri.tryParse(url),
    },
    isLive: live,
  ),
  _ => null,
};

/// The session's state for the player page's [state]: a play or pause
/// control while a page plays, idle otherwise, which ends the session
/// and, on Android, its foreground service.
PlaybackState sessionStateOf(PlaybackUpdate state, {required bool playing}) =>
    switch (state) {
      PlaybackUpdateReady() => PlaybackState(
        controls: [if (playing) MediaControl.pause else MediaControl.play],
        androidCompactActionIndices: const [0],
        processingState: AudioProcessingState.ready,
        playing: playing,
      ),
      _ => PlaybackState(),
    };
