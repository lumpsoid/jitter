import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:jitter/chat/chat_holder.dart';
import 'package:jitter/chat/chat_mode.dart';
import 'package:jitter/chat/chat_panel.dart';
import 'package:jitter/player/desktop_controls.dart';
import 'package:jitter/player/fullscreen.dart';
import 'package:jitter/player/live.dart';
import 'package:jitter/player/live_keys.dart';
import 'package:jitter/player/live_mode_slot.dart';
import 'package:jitter/player/now_playing.dart';
import 'package:jitter/player/playback_failure.dart';
import 'package:jitter/player/playback_holder.dart';
import 'package:jitter/player/playback_loading.dart';
import 'package:jitter/player/playback_title.dart';
import 'package:jitter/player/player_buttons.dart';
import 'package:jitter/player/player_engine.dart';
import 'package:jitter/player/player_layout.dart';
import 'package:jitter/player/player_options.dart';
import 'package:jitter/player/player_shortcuts.dart';
import 'package:jitter/player/position_readout.dart';
import 'package:jitter/player/rewind_messages.dart';
import 'package:jitter/player/seek_bar.dart';
import 'package:jitter/player/sleep_stopped.dart';
import 'package:jitter/player/swipe_down.dart';
import 'package:jitter/player/theatre.dart';
import 'package:jitter/player/touch_controls.dart';
import 'package:jitter/player/video_frame.dart';
import 'package:jitter/player/video_zoom.dart';
import 'package:jitter/player/vod_keys.dart';
import 'package:jitter/player/vod_phone_controls.dart';
import 'package:jitter/player/zoom_gestures.dart';
import 'package:jitter/player/zoom_reset.dart';
import 'package:jitter/player/zoomed_picture.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:jitter/widgets/last_pointer.dart';
import 'package:jitter/widgets/refresh_key.dart';
import 'package:material_ui/material_ui.dart';

/// How often the page reports the player's position; Rust throttles saves.
const _reportEvery = Duration(seconds: 1);

/// How far one wheel notch over the video moves the volume (of 0–100), as
/// media_kit's volume button does.
const double _wheelVolumeStep = 5;

/// The desktop controls' button bar height, as media_kit's.
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
  final PlayerEngine player;

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

  /// Keeps the video's frame, and the picture in it, across fullscreen's
  /// change of layout.
  final GlobalKey _frameKey = GlobalKey();

  /// Whether the page is in theatre: no app bar, the screen fullscreen.
  /// The page's own layout, not Rust's: it is gone when the page closes.
  bool _theatre = false;

  /// Whether the video has the whole screen: no app bar, no chat, the screen
  /// fullscreen. The page's own layout, like theatre's.
  bool _fullscreen = false;

  /// The keyboard's focus on the controls, given at once in fullscreen, so
  /// their keys work: the page's shortcuts hold it otherwise.
  final _controlsFocus = FocusNode();

  /// The zoom into the picture; the page's own, like theatre.
  final ValueNotifier<VideoZoom> _zoom = ValueNotifier(VideoZoom.none);

  @override
  void initState() {
    super.initState();
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

  /// The video's picture under the zoom; a tear-off, equal at each build,
  /// so the video's parameters do not change with it.
  Widget _zoomedPicture(Widget picture) =>
      ZoomedPicture(zoom: _zoom, child: picture);

  /// [controls] under the zoom's gestures and reset, and the rewind's
  /// messages over them.
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

  /// The play/pause button, for either controls.
  PlayPauseButton _playPause({double size = 28}) => PlayPauseButton(
    playing: Live(
      now: () => widget.player.isPlaying,
      changes: () => widget.player.playing,
    ),
    onPressed: () => unawaited(widget.player.togglePlaying()),
    size: size,
  );

  /// The fullscreen button, for either controls.
  FullscreenButton _fullscreenButton({double size = 28}) => FullscreenButton(
    fullscreen: _fullscreen,
    onPressed: _toggleFullscreen,
    size: size,
  );

  /// The volume button, for either controls; it keeps its own wheel.
  VolumeButton _volume() => VolumeButton(
    volumes: widget.player.volumes,
    onToggleMute: () => unawaited(widget.player.toggleMute()),
    onSetVolume: (to) => unawaited(widget.player.setVolume(to)),
    onNudge: (by) => unawaited(widget.player.nudgeVolume(by)),
  );

  /// Clear of the screen's notches and corners in fullscreen, as media_kit's
  /// controls are. [context] must be under theatre's SafeArea, which has
  /// taken its own inset already.
  EdgeInsets _controlsPadding(BuildContext context) =>
      _fullscreen ? MediaQuery.paddingOf(context) : EdgeInsets.zero;

  /// A VOD's controls: the page's own after a touch ([VodPhoneControls]);
  /// after a mouse, [DesktopControls] with the page's seek bar over the
  /// buttons, and the readout and [mode] (shown once rewound) after the
  /// volume. [vodKeys] are its keys. The wheel over the video sets the volume
  /// through the page ([ZoomGestures]); the volume button keeps its own wheel.
  Widget _vodControls(BuildContext context, Widget mode) => _ByLastPointer(
    touch: VodPhoneControls(
      playPause: _playPause(size: 48),
      readout: _readout(fontSize: 14),
      fullscreen: _fullscreenButton(size: 24),
      bar: _seekBar(),
      mode: mode,
      bufferings: () => widget.player.bufferings,
      onSeekBy: (by) => unawaited(widget.player.seekBy(by)),
    ),
    mouse: DesktopControls(
      keys: vodKeys(
        onTogglePlay: () => unawaited(widget.player.togglePlaying()),
        onPlay: () => unawaited(widget.player.play()),
        onPause: () => unawaited(widget.player.pause()),
        onSeekBy: (by) => unawaited(widget.player.seekBy(by)),
        onStepFrame: (forward) =>
            unawaited(widget.player.stepFrame(forward: forward)),
        onNudgeVolume: (by) => unawaited(widget.player.nudgeVolume(by)),
        onToggleFullscreen: _toggleFullscreen,
        onExitFullscreen: _exitFullscreen,
      ),
      barHeight: _buttonBarHeight + seekBarHeight,
      bottomBar: [
        Expanded(
          child: Column(
            children: [
              _seekBar(),
              Expanded(
                child: Row(
                  children: [
                    _playPause(),
                    _volume(),
                    _readout(fontSize: 12),
                    mode,
                    const Spacer(),
                    _fullscreenButton(),
                  ],
                ),
              ),
            ],
          ),
        ),
      ],
      bufferings: () => widget.player.bufferings,
      onToggleFullscreen: _toggleFullscreen,
      padding: _controlsPadding(context),
      focusNode: _controlsFocus,
    ),
    zoom: _zoom,
  );

  /// A live stream's controls: the page's own after a touch ([TouchControls],
  /// with [mode] first on the left); after a mouse, [DesktopControls] with
  /// [mode] after the volume. [liveKeys] are its keys. The wheel over the
  /// video sets the volume through the page, as for a VOD.
  Widget _liveControls(BuildContext context, Widget mode) => _ByLastPointer(
    touch: TouchControls(
      center: _playPause(size: 48),
      bottomBar: [mode, const Spacer(), _fullscreenButton(size: 24)],
      bufferings: () => widget.player.bufferings,
      padding: _controlsPadding(context),
    ),
    mouse: DesktopControls(
      keys: liveKeys(
        onTogglePlay: () => unawaited(widget.player.togglePlaying()),
        onPlay: () => unawaited(widget.player.play()),
        onPause: () => unawaited(widget.player.pause()),
        onNudgeVolume: (by) => unawaited(widget.player.nudgeVolume(by)),
        onToggleFullscreen: _toggleFullscreen,
        onExitFullscreen: _exitFullscreen,
      ),
      bottomBar: [
        _playPause(),
        _volume(),
        mode,
        const Spacer(),
        _fullscreenButton(),
      ],
      bufferings: () => widget.player.bufferings,
      onToggleFullscreen: _toggleFullscreen,
      padding: _controlsPadding(context),
      focusNode: _controlsFocus,
    ),
    zoom: _zoom,
  );

  void _armSleep(int minutes) =>
      ArmSleepTimer(minutes: minutes).sendSignalToRust();

  void _turnOffSleep() => const TurnOffSleepTimer().sendSignalToRust();

  void _toggleFullscreen() {
    setState(() => _fullscreen = !_fullscreen);
    if (_fullscreen) _focusControls();
  }

  /// Gives the keyboard to the controls once the fullscreen layout is built.
  /// Nothing after a touch, when there are no controls to take it.
  void _focusControls() => WidgetsBinding.instance.addPostFrameCallback((_) {
    if (_controlsFocus.context != null) _controlsFocus.requestFocus();
  });

  void _exitFullscreen() => setState(() => _fullscreen = false);

  /// Theatre's key waits in fullscreen: the video has the whole screen, and
  /// leaving theatre would take the window out of fullscreen under it.
  void _toggleTheatre() {
    if (_fullscreen) return;
    setState(() => _theatre = !_theatre);
  }

  /// The chat's key waits in fullscreen too, where there is no chat.
  void _toggleChat() {
    if (_fullscreen) return;
    widget.chat.toggle();
  }

  void _exitTheatre() => setState(() => _theatre = false);

  void _zoomBy(double factor) => _zoom.value = _zoom.value.zoomBy(factor);

  @override
  void dispose() {
    _reporter.cancel();
    _zoom.dispose();
    _controlsFocus.dispose();
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
        onToggleChat: _toggleChat,
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
            final video = VideoFrame(
              key: _frameKey,
              // The video, the failure's Retry or the sleep timer's
              // Resume; not the spinner.
              childTakesFocus: switch (state) {
                PlaybackUpdateReady() ||
                PlaybackUpdateFailed() ||
                PlaybackUpdateSlept() => true,
                _ => false,
              },
              child: _video(state),
            );
            return Scaffold(
              appBar: _theatre || _fullscreen
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
                // A route's Back reaches every PopScope on it, so in
                // fullscreen one Back, or theatre's exit button, leaves
                // fullscreen only.
                onExit: _fullscreen ? _exitFullscreen : _exitTheatre,
                child: Fullscreen(
                  on: _fullscreen,
                  theatre: _theatre,
                  onExit: _exitFullscreen,
                  child: _fullscreen
                      ? ColoredBox(color: const Color(0xFF000000), child: video)
                      : PlayerLayout(
                          // Theatre leaves the video alone, as it does the
                          // app bar. Shown from the open, so the video stays
                          // put when the details come.
                          nowPlaying: _theatre
                              ? null
                              : NowPlaying(
                                  title: details?.title,
                                  game: details?.game,
                                  chapters: details?.chapters ?? const [],
                                  onGoToChapter: _goToChapter,
                                ),
                          video: video,
                          chat: widget.chat.mode == ChatMode.shown
                              ? ChatPanel(chat: widget.chat)
                              : null,
                        ),
                ),
              ),
            );
          },
        ),
      ),
    );
  }

  Widget _video(PlaybackUpdate state) => switch (state) {
    PlaybackUpdateReady(:final live, :final rewind) => Stack(
      fit: StackFit.expand,
      children: [
        widget.player.surface(_zoomedPicture),
        Builder(
          // Under theatre's SafeArea, so the controls' padding is the
          // screen's inset left after it.
          builder: (context) => _zoomable(
            live
                ? _liveControls(context, _mode(rewind))
                : _vodControls(context, _mode(rewind)),
          ),
        ),
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

/// The [touch] controls after a touch, the [mouse] ones after a mouse
/// ([LastPointer]). Under the touch ones a swipe down goes back a level,
/// as Back does: out of fullscreen, else off the page (out of theatre
/// first). Escape goes back as Back does, through the route's
/// EscapeGoesBack; the mouse ones' keys have their own Escape too.
class _ByLastPointer extends StatelessWidget {
  const new({required this.touch, required this.mouse, required this.zoom});

  final Widget touch;
  final Widget mouse;
  final ValueListenable<VideoZoom> zoom;

  /// Back leaves the top level: the PopScopes of [Fullscreen] and [Theatre]
  /// take it.
  void _back(BuildContext context) => Navigator.maybePop(context);

  @override
  Widget build(BuildContext context) {
    if (!LastPointer.isTouch(context)) return mouse;
    return SwipeDown(zoom: zoom, onSwipe: () => _back(context), child: touch);
  }
}
