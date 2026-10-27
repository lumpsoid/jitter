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
import 'package:jitter/player/seek_bar.dart';
import 'package:jitter/player/sleep_stopped.dart';
import 'package:jitter/player/sleep_timer_button.dart';
import 'package:jitter/player/theatre.dart';
import 'package:jitter/player/video_frame.dart';
import 'package:jitter/player/video_zoom.dart';
import 'package:jitter/player/zoom_gestures.dart';
import 'package:jitter/player/zoom_reset.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:media_kit_video/media_kit_video.dart';

/// How often the page reports the player's position; Rust throttles saves.
const _reportEvery = Duration(seconds: 1);

/// How far one wheel notch over the video moves the volume (of 0–100), as
/// media_kit's volume button does.
const double _wheelVolumeStep = 5;

/// media_kit's desktop button bar height, its default.
const double _buttonBarHeight = 56;

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

  /// The zoom into the picture; the page's own, like theatre. Each change
  /// goes to the player, and the page leaves it unzoomed.
  final ValueNotifier<VideoZoom> _zoom = ValueNotifier(VideoZoom.none);

  @override
  void initState() {
    super.initState();
    _zoom.addListener(_showZoom);
    _play();
    _reporter = Timer.periodic(_reportEvery, (_) => _reportPosition());
  }

  /// Asks Rust to load [PlayerPage.target]: on open, and again on Retry
  /// or on the sleep timer's Resume; Rust answers with `PlaybackUpdate`,
  /// loading first.
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

  void _armSleep(int minutes) =>
      ArmSleepTimer(minutes: minutes).sendSignalToRust();

  void _turnOffSleep() => const TurnOffSleepTimer().sendSignalToRust();

  void _toggleTheatre() => setState(() => _theatre = !_theatre);

  void _exitTheatre() => setState(() => _theatre = false);

  void _showZoom() => unawaited(widget.player.setZoom(_zoom.value));

  void _zoomBy(double factor) => _zoom.value = _zoom.value.zoomBy(factor);

  @override
  void dispose() {
    _reporter.cancel();
    _zoom.dispose();
    unawaited(widget.player.setZoom(VideoZoom.none));
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
      onZoomIn: () => _zoomBy(zoomStep),
      onZoomOut: () => _zoomBy(1 / zoomStep),
      onResetZoom: () => _zoom.value = VideoZoom.none,
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
                        :final sleep,
                      )) ...[
                        SleepTimerButton(
                          sleep: sleep,
                          onArm: _armSleep,
                          onTurnOff: _turnOffSleep,
                        ),
                        _QualityMenu(
                          qualities: qualities,
                          selected: selected,
                          onSelected: _selectQuality,
                        ),
                      ],
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
                  // The video, the failure's Retry or the sleep timer's
                  // Resume; not the spinner.
                  childTakesFocus: switch (state) {
                    PlaybackUpdateReady() ||
                    PlaybackUpdateFailed() ||
                    PlaybackUpdateSlept() => true,
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
    PlaybackUpdateReady(:final live) => Stack(
      fit: StackFit.expand,
      children: [
        ZoomGestures(
          zoom: _zoom,
          onWheel: (notches) =>
              unawaited(widget.player.nudgeVolume(notches * _wheelVolumeStep)),
          child: _Controls(
            live: live,
            player: widget.player,
            child: Video(
              key: _videoKey,
              controller: widget.player.controller,
              // Leaving media_kit's fullscreen from theatre goes back to
              // theatre's screen, not out of fullscreen.
              onExitFullscreen: _theatre
                  ? returnToTheatreScreen
                  : defaultExitNativeFullscreen,
            ),
          ),
        ),
        Positioned(top: 8, right: 8, child: ZoomReset(zoom: _zoom)),
      ],
    ),
    PlaybackUpdateFailed(:final message) => PlaybackFailure(
      message: message,
      onRetry: _play,
    ),
    PlaybackUpdateSlept() => SleepStopped(onResume: _play),
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
/// `f` and the arrow keys). A live stream has no seek bar and no position;
/// a VOD on desktop has the page's [SeekBar], which Shift and Ctrl slow.
/// On desktop media_kit's wheel and drag for the volume over the video are
/// off: the page's own wheel sets the volume, through the sleep timer's
/// fade, and the drag moves the zoomed picture. The wheel over the volume
/// button is the button's alone.
class _Controls extends StatelessWidget {
  const new({required this.live, required this.player, required this.child});

  final bool live;
  final MediaKitEngine player;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    if (!live) {
      // media_kit's own buttons, the volume button keeping its wheel, under
      // the page's seek bar: the bar is part of the button bar, so a click
      // on it is not a click on the video.
      final desktop = MaterialDesktopVideoControlsThemeData(
        modifyVolumeOnScroll: false,
        displaySeekBar: false,
        buttonBarHeight: _buttonBarHeight + seekBarHeight,
        bottomButtonBar: [
          Expanded(
            child: Column(
              children: [
                SeekBar(
                  positions: () => player.positions,
                  durations: () => player.durations,
                  buffers: () => player.buffers,
                  onSeek: (to) => unawaited(player.seek(to)),
                ),
                const Expanded(
                  child: Row(
                    children: [
                      MaterialDesktopSkipPreviousButton(),
                      MaterialDesktopPlayOrPauseButton(),
                      MaterialDesktopSkipNextButton(),
                      KeepsWheel(child: MaterialDesktopVolumeButton()),
                      MaterialDesktopPositionIndicator(),
                      Spacer(),
                      MaterialDesktopFullscreenButton(),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ],
      );
      return MaterialDesktopVideoControlsTheme(
        normal: desktop,
        fullscreen: desktop,
        child: child,
      );
    }
    const mobile = MaterialVideoControlsThemeData(
      displaySeekBar: false,
      bottomButtonBar: [Spacer(), MaterialFullscreenButton()],
    );
    const desktop = MaterialDesktopVideoControlsThemeData(
      displaySeekBar: false,
      modifyVolumeOnScroll: false,
      bottomButtonBar: [
        MaterialDesktopPlayOrPauseButton(),
        KeepsWheel(child: MaterialDesktopVolumeButton()),
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
