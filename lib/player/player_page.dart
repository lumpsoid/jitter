import 'dart:async';

import 'package:flutter/material.dart';
import 'package:jitter/player/media_kit_engine.dart';
import 'package:jitter/player/playback_holder.dart';
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
    super.key,
  });

  /// What to play; every report to Rust is tagged with it.
  final PlaybackTarget target;

  /// Shown in the app bar: the channel's or the VOD's name.
  final String title;

  final PlaybackHolder playback;
  final MediaKitEngine player;

  @override
  State<PlayerPage> createState() => _PlayerPageState();
}

class _PlayerPageState extends State<PlayerPage> {
  late final Timer _reporter;

  @override
  void initState() {
    super.initState();
    Play(target: widget.target).sendSignalToRust();
    _reporter = Timer.periodic(_reportEvery, (_) => _reportPosition());
  }

  void _reportPosition() => PlayerPosition(
    target: widget.target,
    positionS: widget.player.position.inSeconds,
    durationS: widget.player.duration.inSeconds,
  ).sendSignalToRust();

  void _selectQuality(String name) => SelectQuality(
    name: name,
    positionS: widget.player.position.inSeconds,
  ).sendSignalToRust();

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
    return ListenableBuilder(
      listenable: widget.playback,
      builder: (context, _) {
        final state = widget.playback.state;
        return Scaffold(
          appBar: AppBar(
            title: Text(widget.title),
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
            ],
          ),
          // The video alone for now; a chat panel goes beside it (wide) or
          // below it (narrow).
          body: Center(
            child: AspectRatio(
              aspectRatio: 16 / 9,
              child: ColoredBox(color: Colors.black, child: _video(state)),
            ),
          ),
        );
      },
    );
  }

  Widget _video(PlaybackUpdate state) => switch (state) {
    PlaybackUpdateReady(:final live) => _Controls(
      live: live,
      child: Video(controller: widget.player.controller),
    ),
    PlaybackUpdateFailed(:final message) => Center(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Text(
          message,
          textAlign: TextAlign.center,
          style: const TextStyle(color: Colors.white),
        ),
      ),
    ),
    _ => const Center(child: CircularProgressIndicator()),
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
