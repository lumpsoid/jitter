/// The player's own buttons, in media_kit's look: play/pause, volume and
/// fullscreen, drawn as media_kit draws its own. They take narrow values and
/// callbacks, not the engine, so the page wires them to `PlayerEngine`.
library;

import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:jitter/player/live.dart';
import 'package:jitter/player/zoom_gestures.dart';
import 'package:material_ui/material_ui.dart';

const _white = Color(0xFFFFFFFF);

/// How long the play/pause icon takes to turn into the other.
const _playPauseDuration = Duration(milliseconds: 200);

/// How long the volume icon takes to change, and the slider to show.
const _volumeDuration = Duration(milliseconds: 150);

/// How far one wheel notch moves the volume, of 0–100.
const _wheelStep = 5.0;

/// The slider's room beside the volume icon: a lead, the slider, a tail.
const _lead = 12.0;
const _sliderWidth = 52.0;
const _tail = 18.0;

/// media_kit's play/pause button: its icon turns from play into pause, or
/// back, as [playings] say the player plays or pauses. It starts where
/// [playing] is, with no animation on mount.
class PlayPauseButton extends StatefulWidget {
  const new({
    required this.playing,
    required this.playings,
    required this.onPressed,
    this.size = 28,
    super.key,
  });

  final bool playing;

  /// Whether it plays now, then each change; a new stream each call.
  final Stream<bool> Function() playings;

  final VoidCallback onPressed;

  final double size;

  @override
  State<PlayPauseButton> createState() => _PlayPauseButtonState();
}

class _PlayPauseButtonState extends State<PlayPauseButton>
    with SingleTickerProviderStateMixin {
  late final AnimationController _animation;
  late final StreamSubscription<bool> _playings;

  @override
  void initState() {
    super.initState();
    _animation = AnimationController(
      vsync: this,
      value: widget.playing ? 1 : 0,
      duration: _playPauseDuration,
    );
    _playings = widget.playings().listen((playing) {
      if (playing) {
        _animation.forward();
      } else {
        _animation.reverse();
      }
    });
  }

  @override
  void dispose() {
    unawaited(_playings.cancel());
    _animation.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return IconButton(
      onPressed: widget.onPressed,
      iconSize: widget.size,
      color: _white,
      icon: AnimatedIcon(
        progress: _animation,
        icon: AnimatedIcons.play_pause,
        size: widget.size,
        color: _white,
      ),
    );
  }
}

/// media_kit's fullscreen button: the expand icon, or the one to leave it
/// when [fullscreen].
class FullscreenButton extends StatelessWidget {
  const new({
    required this.fullscreen,
    required this.onPressed,
    this.size = 28,
    super.key,
  });

  final bool fullscreen;

  final VoidCallback onPressed;

  final double size;

  @override
  Widget build(BuildContext context) {
    return IconButton(
      onPressed: onPressed,
      iconSize: size,
      color: _white,
      icon: Icon(fullscreen ? Icons.fullscreen_exit : Icons.fullscreen),
    );
  }
}

/// media_kit's volume button: an icon for the level, and on hover a slider
/// beside it. A click toggles mute, a drag on the slider sets the volume,
/// and the wheel nudges it by 5. Each goes to the engine's fade-aware call,
/// so a sleep timer's fade is not fought.
///
/// The volume is what [volumes] says, now and then each change (0–100).
class VolumeButton extends StatefulWidget {
  const new({
    required this.volumes,
    required this.onToggleMute,
    required this.onSetVolume,
    required this.onNudge,
    this.size = 28,
    super.key,
  });

  final Live<double> volumes;

  final VoidCallback onToggleMute;

  /// Sets the volume to the value dragged to, of 0–100.
  final ValueChanged<double> onSetVolume;

  /// Moves the volume by the wheel's step: up as positive.
  final ValueChanged<double> onNudge;

  final double size;

  @override
  State<VolumeButton> createState() => _VolumeButtonState();
}

class _VolumeButtonState extends State<VolumeButton> {
  late double _volume;
  late final StreamSubscription<double> _changes;
  bool _hovered = false;

  @override
  void initState() {
    super.initState();
    _volume = widget.volumes.now();
    _changes = widget.volumes.changes().listen(
      (volume) => setState(() => _volume = volume),
    );
  }

  @override
  void dispose() {
    unawaited(_changes.cancel());
    super.dispose();
  }

  void _onWheel(PointerSignalEvent event) {
    if (event is! PointerScrollEvent) return;
    if (event.scrollDelta.dy < 0) widget.onNudge(_wheelStep);
    if (event.scrollDelta.dy > 0) widget.onNudge(-_wheelStep);
  }

  IconData get _icon {
    if (_volume == 0) return Icons.volume_off;
    if (_volume < 50) return Icons.volume_down;
    return Icons.volume_up;
  }

  @override
  Widget build(BuildContext context) {
    final icon = _icon;
    return KeepsWheel(
      child: MouseRegion(
        onEnter: (_) => setState(() => _hovered = true),
        onExit: (_) => setState(() => _hovered = false),
        child: Listener(
          onPointerSignal: _onWheel,
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              const SizedBox(width: 4),
              IconButton(
                onPressed: widget.onToggleMute,
                iconSize: widget.size * 0.8,
                color: _white,
                icon: AnimatedSwitcher(
                  duration: _volumeDuration,
                  child: Icon(icon, key: ValueKey(icon)),
                ),
              ),
              AnimatedOpacity(
                opacity: _hovered ? 1 : 0,
                duration: _volumeDuration,
                child: AnimatedContainer(
                  width: _hovered ? _lead + _sliderWidth + _tail : _lead,
                  duration: _volumeDuration,
                  child: SingleChildScrollView(
                    scrollDirection: Axis.horizontal,
                    child: Row(
                      children: [
                        const SizedBox(width: _lead),
                        SizedBox(width: _sliderWidth, child: _slider()),
                        const SizedBox(width: _tail),
                      ],
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _slider() {
    return SliderTheme(
      data: SliderThemeData(
        trackHeight: 1.2,
        inactiveTrackColor: const Color(0x3DFFFFFF),
        activeTrackColor: _white,
        thumbColor: _white,
        thumbShape: const RoundSliderThumbShape(
          enabledThumbRadius: 6,
          elevation: 0,
          pressedElevation: 0,
        ),
        trackShape: _CustomTrackShape(),
        overlayColor: const Color(0x00000000),
      ),
      child: Slider(
        value: _volume.clamp(0, 100),
        min: 0,
        max: 100,
        onChanged: widget.onSetVolume,
      ),
    );
  }
}

/// A track the full width of its slider, centred across its height, with
/// no inset at either end as the slider's default track has.
class _CustomTrackShape extends RoundedRectSliderTrackShape {
  @override
  Rect getPreferredRect({
    required RenderBox parentBox,
    required SliderThemeData sliderTheme,
    Offset offset = Offset.zero,
    bool isEnabled = false,
    bool isDiscrete = false,
  }) {
    final height = sliderTheme.trackHeight;
    final left = offset.dx;
    final top = offset.dy + (parentBox.size.height - height!) / 2;
    final width = parentBox.size.width;
    return Rect.fromLTWH(left, top, width, height);
  }
}
