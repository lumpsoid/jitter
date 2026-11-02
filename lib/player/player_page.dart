import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:jitter/chat/chat_holder.dart';
import 'package:jitter/chat/chat_mode.dart';
import 'package:jitter/chat/chat_panel.dart';
import 'package:jitter/player/live_mode_slot.dart';
import 'package:jitter/player/media_kit_engine.dart';
import 'package:jitter/player/now_playing.dart';
import 'package:jitter/player/playback_failure.dart';
import 'package:jitter/player/playback_holder.dart';
import 'package:jitter/player/playback_loading.dart';
import 'package:jitter/player/playback_title.dart';
import 'package:jitter/player/player_layout.dart';
import 'package:jitter/player/player_options.dart';
import 'package:jitter/player/player_shortcuts.dart';
import 'package:jitter/player/position_readout.dart';
import 'package:jitter/player/rewind_messages.dart';
import 'package:jitter/player/seek_bar.dart';
import 'package:jitter/player/sleep_stopped.dart';
import 'package:jitter/player/swipe_down.dart';
import 'package:jitter/player/theatre.dart';
import 'package:jitter/player/video_frame.dart';
import 'package:jitter/player/video_zoom.dart';
import 'package:jitter/player/vod_keys.dart';
import 'package:jitter/player/vod_phone_controls.dart';
import 'package:jitter/player/zoom_gestures.dart';
import 'package:jitter/player/zoom_reset.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:jitter/widgets/escape_goes_back.dart';
import 'package:jitter/widgets/last_pointer.dart';
import 'package:jitter/widgets/refresh_key.dart';
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
  /// the app bar's options can hide and show it.
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

  /// Ctrl+R: a page playing shows what plays afresh, the player and the
  /// chat left as they are; a failed page tries again, as its Retry does.
  void _refresh() => switch (widget.playback.state) {
    PlaybackUpdateReady() => RefreshPlayback(
      target: widget.target,
    ).sendSignalToRust(),
    PlaybackUpdateFailed() => _play(),
    _ => null,
  };

  void _reportPosition() => PlayerPosition(
    target: widget.target,
    positionS: widget.player.position.inSeconds,
    durationS: widget.player.duration.inSeconds,
  ).sendSignalToRust();

  void _selectQuality(String name) => SelectQuality(
    name: name,
    positionS: widget.player.position.inSeconds,
  ).sendSignalToRust();

  /// Asks Rust to read a typed time and seek there; the dialog it was
  /// typed in hears the answer.
  void _goTo(String text) => GoToTime(
    text: text,
    durationS: widget.player.duration.inSeconds,
  ).sendSignalToRust();

  /// Asks Rust to seek to the start of a VOD's chapter.
  void _goToChapter(int index) => GoToChapter(index: index).sendSignalToRust();

  /// Asks Rust to rewind the live stream into its recording; Rust answers
  /// with `PlaybackUpdate`, or with a `RewindAnswer` when it can't.
  void _rewind() => RewindStream(target: widget.target).sendSignalToRust();

  /// Asks Rust to go from the rewound recording back to the live edge.
  void _backToLive() => BackToLive(target: widget.target).sendSignalToRust();

  /// A live stream's mode, for either controls; nothing for a VOD.
  LiveModeSlot _mode(PlaybackRewind rewind) => LiveModeSlot(
    rewind: rewind,
    onRewind: _rewind,
    onBackToLive: _backToLive,
  );

  /// A VOD's seek bar, for either controls.
  SeekBar _seekBar() => SeekBar(
    positions: widget.player.positions,
    durations: widget.player.durations,
    buffers: widget.player.buffers,
    onSeek: (to) => unawaited(widget.player.seek(to)),
  );

  /// A VOD's readout, in media_kit's size: smaller on desktop.
  PositionReadout _readout({required double fontSize}) => PositionReadout(
    positions: widget.player.positions,
    durations: widget.player.durations,
    answers: () => GoToTimeAnswer.rustSignalStream.map((pack) => pack.message),
    onGoTo: _goTo,
    fontSize: fontSize,
  );

  /// [controls] under the zoom's gestures and reset, and the rewind's
  /// messages over them: within media_kit's controls, so its fullscreen
  /// route, which builds only the controls anew, has them too.
  Widget _zoomable(Widget controls) => RewindMessages(
    answers: () => RewindAnswer.rustSignalStream.map((pack) => pack.message),
    child: Stack(
      fit: StackFit.expand,
      children: [
        ZoomGestures(
          zoom: _zoom,
          onWheel: (notches) =>
              unawaited(widget.player.nudgeVolume(notches * _wheelVolumeStep)),
          child: controls,
        ),
        Positioned(top: 8, right: 8, child: ZoomReset(zoom: _zoom)),
      ],
    ),
  );

  /// A VOD's controls: the page's own after a touch ([VodPhoneControls]),
  /// media_kit's desktop ones themed by [_Controls] after a mouse.
  Widget _vodControls(VideoState video, Widget mode) => _ByLastPointer(
    touch: VodPhoneControls(
      playPause: const MaterialPlayOrPauseButton(iconSize: 48),
      readout: _readout(fontSize: 14),
      fullscreen: const MaterialFullscreenButton(),
      bar: _seekBar(),
      mode: mode,
      bufferings: () => widget.player.bufferings,
      onSeekBy: (by) => unawaited(widget.player.seekBy(by)),
    ),
    mouse: MaterialDesktopVideoControls(video),
    zoom: _zoom,
  );

  /// A live stream's controls: media_kit's mobile ones after a touch, its
  /// desktop ones after a mouse, both themed by [_Controls].
  Widget _liveControls(VideoState video) => _ByLastPointer(
    touch: MaterialVideoControls(video),
    mouse: MaterialDesktopVideoControls(video),
    zoom: _zoom,
  );

  void _armSleep(int minutes) =>
      ArmSleepTimer(minutes: minutes).sendSignalToRust();

  void _turnOffSleep() => const TurnOffSleepTimer().sendSignalToRust();

  void _toggleFullscreen() =>
      unawaited(_videoKey.currentState?.toggleFullscreen());

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
    return RefreshKey(
      onRefresh: _refresh,
      child: PlayerShortcuts(
        onToggleChat: widget.chat.toggle,
        onToggleMute: () => unawaited(widget.player.toggleMute()),
        onToggleFullscreen: _toggleFullscreen,
        onTogglePlay: () => unawaited(widget.player.togglePlaying()),
        onToggleTheatre: _toggleTheatre,
        onZoomIn: () => _zoomBy(zoomStep),
        onZoomOut: () => _zoomBy(1 / zoomStep),
        onResetZoom: () => _zoom.value = VideoZoom.none,
        child: ListenableBuilder(
          listenable: Listenable.merge([widget.playback, widget.chat]),
          builder: (context, _) {
            final state = widget.playback.state;
            final details = switch (state) {
              PlaybackUpdateReady(:final details) => details,
              _ => null,
            };
            return Scaffold(
              appBar: _theatre
                  ? null
                  : AppBar(
                      title: PlaybackTitle(
                        title: widget.title,
                        onOpenChannel: widget.onOpenChannel,
                        details: details,
                      ),
                      actions: [
                        PlayerOptions(
                          playback: widget.playback,
                          chat: widget.chat,
                          onSelectQuality: _selectQuality,
                          onArmSleep: _armSleep,
                          onTurnOffSleep: _turnOffSleep,
                          onToggleTheatre: _toggleTheatre,
                        ),
                      ],
                    ),
              body: Theatre(
                on: _theatre,
                onExit: _exitTheatre,
                child: PlayerLayout(
                  // Theatre leaves the video alone, as it does the app bar.
                  // Shown from the open, so the video stays put when the
                  // details come.
                  nowPlaying: _theatre
                      ? null
                      : NowPlaying(
                          title: details?.title,
                          game: details?.game,
                          chapters: details?.chapters ?? const [],
                          onGoToChapter: _goToChapter,
                        ),
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
      ),
    );
  }

  Widget _video(PlaybackUpdate state) => switch (state) {
    PlaybackUpdateReady(:final live, :final rewind) => Video(
      key: _videoKey,
      controller: widget.player.controller,
      // The theme sits inside the controls, not around the video:
      // media_kit's fullscreen route copies the theme around the video
      // once, on entering, but builds the controls anew at each change,
      // and a rewind turns a live stream's controls into a VOD's.
      controls: (video) => _zoomable(
        _Controls(
          live: live,
          player: widget.player,
          mode: _mode(rewind),
          bar: _seekBar(),
          readout: _readout(fontSize: 12),
          onToggleFullscreen: _toggleFullscreen,
          onExitFullscreen: () =>
              unawaited(_videoKey.currentState?.exitFullscreen()),
          child: live
              ? _liveControls(video)
              : _vodControls(video, _mode(rewind)),
        ),
      ),
      // Leaving media_kit's fullscreen from theatre goes back to
      // theatre's screen, not out of fullscreen.
      onExitFullscreen: _theatre
          ? returnToTheatreScreen
          : defaultExitNativeFullscreen,
    ),
    PlaybackUpdateFailed(:final message) => PlaybackFailure(
      message: message,
      onRetry: _play,
    ),
    PlaybackUpdateSlept() => SleepStopped(onResume: _play),
    _ => const PlaybackLoading(),
  };
}

/// media_kit's controls, themed (play/pause, volume, fullscreen; on desktop
/// space, `f` and the arrow keys, a VOD's as [vodKeys] has them). A live
/// stream has no seek bar and no position. A VOD after a mouse has the
/// page's [SeekBar] and [PositionReadout] among media_kit's buttons; after
/// a touch it has the page's own controls instead ([VodPhoneControls]). A live
/// stream's [LiveModeSlot] sits left, after volume, or after the readout
/// once rewound (`live` is false then, so it has a VOD's controls).
/// On desktop media_kit's wheel and drag for the volume over the video are
/// off: the page's own wheel sets the volume, through the sleep timer's
/// fade, and the drag moves the zoomed picture. The wheel over the volume
/// button is the button's alone.
class _Controls extends StatelessWidget {
  const new({
    required this.live,
    required this.player,
    required this.mode,
    required this.bar,
    required this.readout,
    required this.onToggleFullscreen,
    required this.onExitFullscreen,
    required this.child,
  });

  final bool live;
  final MediaKitEngine player;

  /// A live stream's [LiveModeSlot]; renders nothing for a VOD.
  final Widget mode;

  /// A VOD's seek bar and readout, for desktop.
  final Widget bar;
  final Widget readout;

  /// For a VOD's keys, `f` and Escape: media_kit's own go through the
  /// controls, which a key map cannot reach.
  final VoidCallback onToggleFullscreen;
  final VoidCallback onExitFullscreen;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    if (!live) {
      // media_kit's own buttons, with the page's seek bar over them as
      // part of the button bar, so a click on it is not a click on the
      // video; the volume button keeps its wheel.
      final desktop = MaterialDesktopVideoControlsThemeData(
        modifyVolumeOnScroll: false,
        displaySeekBar: false,
        keyboardShortcuts: vodKeys(
          onTogglePlay: () => unawaited(player.togglePlaying()),
          onPlay: () => unawaited(player.play()),
          onPause: () => unawaited(player.pause()),
          onSeekBy: (by) => unawaited(player.seekBy(by)),
          onStepFrame: (forward) =>
              unawaited(player.stepFrame(forward: forward)),
          onNudgeVolume: (by) => unawaited(player.nudgeVolume(by)),
          onToggleFullscreen: onToggleFullscreen,
          onExitFullscreen: onExitFullscreen,
        ),
        buttonBarHeight: _buttonBarHeight + seekBarHeight,
        bottomButtonBar: [
          Expanded(
            child: Column(
              children: [
                bar,
                Expanded(
                  child: Row(
                    children: [
                      const MaterialDesktopSkipPreviousButton(),
                      const MaterialDesktopPlayOrPauseButton(),
                      const MaterialDesktopSkipNextButton(),
                      const KeepsWheel(child: MaterialDesktopVolumeButton()),
                      readout,
                      mode,
                      const Spacer(),
                      const MaterialDesktopFullscreenButton(),
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
    final mobile = MaterialVideoControlsThemeData(
      displaySeekBar: false,
      bottomButtonBar: [mode, const Spacer(), const MaterialFullscreenButton()],
    );
    final desktop = MaterialDesktopVideoControlsThemeData(
      displaySeekBar: false,
      modifyVolumeOnScroll: false,
      bottomButtonBar: [
        const MaterialDesktopPlayOrPauseButton(),
        const KeepsWheel(child: MaterialDesktopVolumeButton()),
        mode,
        const Spacer(),
        const MaterialDesktopFullscreenButton(),
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

/// The [touch] controls after a touch, the [mouse] ones after a mouse
/// ([LastPointer]). Under the touch ones a swipe down goes back a level,
/// as Back does: out of media_kit's fullscreen, else off the page (out of
/// theatre first). In fullscreen Escape leaves it too: the mouse ones'
/// keys have their own Escape, the touch ones have no keys.
class _ByLastPointer extends StatelessWidget {
  const new({required this.touch, required this.mouse, required this.zoom});

  final Widget touch;
  final Widget mouse;
  final ValueListenable<VideoZoom> zoom;

  void _back(BuildContext context) => unawaited(
    isFullscreen(context)
        ? exitFullscreen(context)
        : Navigator.maybePop(context),
  );

  @override
  Widget build(BuildContext context) {
    if (!LastPointer.isTouch(context)) return mouse;
    final swipeable = SwipeDown(
      zoom: zoom,
      onSwipe: () => _back(context),
      child: touch,
    );
    if (!isFullscreen(context)) return swipeable;
    return EscapeGoesBack(onEscape: () => _back(context), child: swipeable);
  }
}
