import 'dart:async';

import 'package:flutter/material.dart';
import 'package:jitter/chat/chat_holder.dart';
import 'package:jitter/chat/chat_mode.dart';
import 'package:jitter/chat/chat_panel.dart';
import 'package:jitter/chat/chat_toggle.dart';
import 'package:jitter/player/media_kit_engine.dart';
import 'package:jitter/player/playback_failure.dart';
import 'package:jitter/player/playback_holder.dart';
import 'package:jitter/player/playback_loading.dart';
import 'package:jitter/player/playback_title.dart';
import 'package:jitter/player/player_layout.dart';
import 'package:jitter/player/player_shortcuts.dart';
import 'package:jitter/player/theatre.dart';
import 'package:jitter/player/video_frame.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:media_kit_video/media_kit_video.dart';

/// How often the page reports the player's position; Rust throttles saves.
const _reportEvery = Duration(seconds: 1);

/// Plays [target]: sends `Play` when it opens and `LeavePlayer` when it
/// closes; Rust opens and stops the player. Push it with `Navigator.push`.
class PlayerPage extends StatefulWidget {
  const new({
    required this.target,
    required this.title,
    required this.playback,
    required this.player,
    required this.chat,
    required this.onOpenChannel,
    super.key,
  });

  /// What to play; every report to Rust is tagged with it.
  final PlaybackTarget target;

  /// Shown in the app bar: the channel's or the VOD's name.
  final String title;

  final PlaybackHolder playback;
  final MediaKitEngine player;

  /// The chat (live, or a VOD's replay), shown while Rust has one open;
  /// the app bar can hide and show it.
  final ChatHolder chat;

  /// Opens the channel being watched (a VOD's owner) by login, from the
  /// app bar title.
  final ValueChanged<String> onOpenChannel;

  @override
  State<PlayerPage> createState() => _PlayerPageState();
}

class _PlayerPageState extends State<PlayerPage> {
  late final Timer _reporter;

  /// The video, for the fullscreen key; unattached until playback is ready.
  final _videoKey = GlobalKey<VideoState>();

  /// Whether the page is in theatre: no app bar, the screen fullscreen.
  /// The page's own layout, not Rust's: it is gone when the page closes.
  bool _theatre = false;

  @override
  void initState() {
    super.initState();
    _play();
    _reporter = Timer.periodic(_reportEvery, (_) => _reportPosition());
  }

  /// Asks Rust to load [PlayerPage.target]: on open, and again on Retry;
  /// Rust answers with `PlaybackUpdate`, loading first.
  void _play() => Play(target: widget.target).sendSignalToRust();

  void _reportPosition() => PlayerPosition(
    target: widget.target,
    positionS: widget.player.position.inSeconds,
    durationS: widget.player.duration.inSeconds,
  ).sendSignalToRust();

  void _selectQuality(String name) => SelectQuality(
    name: name,
    positionS: widget.player.position.inSeconds,
  ).sendSignalToRust();

  void _toggleTheatre() => setState(() => _theatre = !_theatre);

  void _exitTheatre() => setState(() => _theatre = false);

  @override
  void dispose() {
    _reporter.cancel();
    LeavePlayer(
      target: widget.target,
      positionS: widget.player.position.inSeconds,
      durationS: widget.player.duration.inSeconds,
    ).sendSignalToRust();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return PlayerShortcuts(
      onToggleChat: widget.chat.toggle,
      onToggleMute: () => unawaited(widget.player.toggleMute()),
      onToggleFullscreen: () =>
          unawaited(_videoKey.currentState?.toggleFullscreen()),
      onTogglePlay: () => unawaited(widget.player.togglePlaying()),
      onToggleTheatre: _toggleTheatre,
      child: ListenableBuilder(
        listenable: Listenable.merge([widget.playback, widget.chat]),
        builder: (context, _) {
          final state = widget.playback.state;
          return Scaffold(
            appBar: _theatre
                ? null
                : AppBar(
                    title: PlaybackTitle(
                      title: widget.title,
                      onOpenChannel: widget.onOpenChannel,
                      details: switch (state) {
                        PlaybackUpdateReady(:final details) => details,
                        _ => null,
                      },
                    ),
                    actions: [
                      if (state case PlaybackUpdateReady(
                        :final qualities,
                        :final selected,
                      ))
                        _QualityMenu(
                          qualities: qualities,
                          selected: selected,
                          onSelected: _selectQuality,
                        ),
                      ChatToggle(chat: widget.chat),
                      IconButton(
                        tooltip: 'Theatre mode',
                        icon: const Icon(Icons.fit_screen_outlined),
                        onPressed: _toggleTheatre,
                      ),
                    ],
                  ),
            body: Theatre(
              on: _theatre,
              onExit: _exitTheatre,
              child: PlayerLayout(
                video: VideoFrame(
                  // The video, or the failure's Retry; not the spinner.
                  childTakesFocus: switch (state) {
                    PlaybackUpdateReady() || PlaybackUpdateFailed() => true,
                    _ => false,
                  },
                  child: _video(state),
                ),
                chat: widget.chat.mode == ChatMode.shown
                    ? ChatPanel(chat: widget.chat)
                    : null,
              ),
            ),
          );
        },
      ),
    );
  }

  Widget _video(PlaybackUpdate state) => switch (state) {
    PlaybackUpdateReady(:final live) => _Controls(
      live: live,
      child: Video(
        key: _videoKey,
        controller: widget.player.controller,
        // Leaving media_kit's fullscreen from theatre goes back to theatre's
        // screen, not out of fullscreen.
        onExitFullscreen: _theatre
            ? returnToTheatreScreen
            : defaultExitNativeFullscreen,
      ),
    ),
    PlaybackUpdateFailed(:final message) => PlaybackFailure(
      message: message,
      onRetry: _play,
    ),
    _ => const PlaybackLoading(),
  };
}

/// The quality menu: Rust's labels, the selected one checked.
class _QualityMenu extends StatelessWidget {
  const new({
    required this.qualities,
    required this.selected,
    required this.onSelected,
  });

  final List<QualityOption> qualities;
  final String selected;
  final ValueChanged<String> onSelected;

  @override
  Widget build(BuildContext context) {
    return PopupMenuButton<String>(
      icon: const Icon(Icons.high_quality_outlined),
      tooltip: 'Quality',
      onSelected: onSelected,
      itemBuilder: (context) => [
        for (final q in qualities)
          CheckedPopupMenuItem(
            value: q.name,
            checked: q.name == selected,
            child: Text(q.label),
          ),
      ],
    );
  }
}

/// media_kit's controls (play/pause, volume, fullscreen; on desktop space,
/// `f` and the arrow keys). A live stream has no seek bar and no position.
class _Controls extends StatelessWidget {
  const new({required this.live, required this.child});

  final bool live;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    if (!live) return child;
    const mobile = MaterialVideoControlsThemeData(
      displaySeekBar: false,
      bottomButtonBar: [Spacer(), MaterialFullscreenButton()],
    );
    const desktop = MaterialDesktopVideoControlsThemeData(
      displaySeekBar: false,
      bottomButtonBar: [
        MaterialDesktopPlayOrPauseButton(),
        MaterialDesktopVolumeButton(),
        Spacer(),
        MaterialDesktopFullscreenButton(),
      ],
    );
    return MaterialVideoControlsTheme(
      normal: mobile,
      fullscreen: mobile,
      child: MaterialDesktopVideoControlsTheme(
        normal: desktop,
        fullscreen: desktop,
        child: child,
      ),
    );
  }
}
