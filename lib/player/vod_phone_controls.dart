import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:jitter/player/vod_keys.dart';
import 'package:media_kit_video/media_kit_video.dart';

/// How long the controls stay after the last finger lifts.
const _hideAfter = Duration(seconds: 3);

const _fade = Duration(milliseconds: 300);

/// How long a double tap's `−10 s` or `+10 s` shows.
const _flashFor = Duration(milliseconds: 700);

const _backdrop = Color(0x66000000);
const _white = Color(0xFFFFFFFF);

/// A VOD's controls on a phone, in place of media_kit's. A tap shows them
/// over the dimmed picture and another hides them, as do 3 s after the
/// last finger lifts. A finger down anywhere on them holds them: a long,
/// slow drag on the [bar] keeps it, where media_kit's hide 3 s after the
/// tap that showed them, drag or not. A double tap on the left or right
/// third goes [vodSeekStep] back or on, shown or not, and flashes it.
///
/// The parts are given: media_kit's [playPause] and [fullscreen] buttons,
/// the page's [readout] and seek [bar]; these controls only show and hide
/// them.
class VodPhoneControls extends StatefulWidget {
  const new({
    required this.playPause,
    required this.readout,
    required this.fullscreen,
    required this.bar,
    required this.bufferings,
    required this.onSeekBy,
    super.key,
  });

  final Widget playPause;
  final Widget readout;
  final Widget fullscreen;
  final Widget bar;

  /// Whether the player waits for the VOD to load: now, then each change;
  /// a new stream each call. The spinner shows in place of [playPause].
  final Stream<bool> Function() bufferings;

  /// Moves the VOD by a double tap's step.
  final ValueChanged<Duration> onSeekBy;

  @override
  State<VodPhoneControls> createState() => _VodPhoneControlsState();
}

class _VodPhoneControlsState extends State<VodPhoneControls> {
  /// Whether the controls are shown, or fading in.
  bool _visible = false;

  /// Whether they are built: through the fade out too, then not, so a
  /// hidden bar does not rebuild at each position.
  bool _built = false;

  Timer? _hide;

  /// The fingers down on the controls; while any is, they stay.
  int _down = 0;

  bool _buffering = false;
  late final StreamSubscription<bool> _bufferings;

  /// Where the last double tap went down.
  Offset? _doubleTapAt;

  /// The step a double tap just went, shown on its side; null when none.
  Duration? _flash;
  Timer? _flashing;

  @override
  void initState() {
    super.initState();
    _bufferings = widget.bufferings().listen(
      (buffering) => setState(() => _buffering = buffering),
    );
  }

  @override
  void dispose() {
    _hide?.cancel();
    _flashing?.cancel();
    unawaited(_bufferings.cancel());
    super.dispose();
  }

  void _show() {
    setState(() {
      _visible = true;
      _built = true;
    });
    _hideLater();
  }

  void _hideNow() {
    _hide?.cancel();
    setState(() => _visible = false);
  }

  /// Hides them [_hideAfter] from now, unless a finger is down.
  void _hideLater() {
    _hide?.cancel();
    if (_visible && _down == 0) _hide = Timer(_hideAfter, _hideNow);
  }

  void _onDown(PointerDownEvent _) {
    _down++;
    _hide?.cancel();
  }

  void _onUp(PointerEvent _) {
    _down = math.max(0, _down - 1);
    _hideLater();
  }

  void _onDoubleTap(double width) {
    final x = _doubleTapAt?.dx;
    if (x == null || width <= 0) return;
    final step = x < width / 3
        ? -vodSeekStep
        : x > width * 2 / 3
        ? vodSeekStep
        : null;
    if (step == null) return;
    widget.onSeekBy(step);
    _flashing?.cancel();
    setState(() => _flash = step);
    _flashing = Timer(_flashFor, () => setState(() => _flash = null));
  }

  @override
  Widget build(BuildContext context) {
    final flash = _flash;
    return LayoutBuilder(
      builder: (context, constraints) => Listener(
        onPointerDown: _onDown,
        onPointerUp: _onUp,
        onPointerCancel: _onUp,
        child: Stack(
          fit: StackFit.expand,
          children: [
            GestureDetector(
              behavior: HitTestBehavior.opaque,
              onTap: () => _visible ? _hideNow() : _show(),
              onDoubleTapDown: (details) =>
                  _doubleTapAt = details.localPosition,
              onDoubleTap: () => _onDoubleTap(constraints.maxWidth),
            ),
            if (_built)
              AnimatedOpacity(
                opacity: _visible ? 1 : 0,
                duration: _fade,
                onEnd: () {
                  if (!_visible) setState(() => _built = false);
                },
                child: IgnorePointer(ignoring: !_visible, child: _controls()),
              ),
            if (_buffering)
              const IgnorePointer(
                child: Center(child: CircularProgressIndicator(color: _white)),
              ),
            if (flash != null)
              IgnorePointer(
                child: Align(
                  alignment: flash.isNegative
                      ? const Alignment(-0.6, 0)
                      : const Alignment(0.6, 0),
                  child: _Flash(step: flash),
                ),
              ),
          ],
        ),
      ),
    );
  }

  /// The backdrop lets a tap through to hide them; only the parts take
  /// one.
  Widget _controls() => Stack(
    children: [
      const Positioned.fill(
        child: IgnorePointer(child: ColoredBox(color: _backdrop)),
      ),
      if (!_buffering) Center(child: widget.playPause),
      Positioned(
        left: 0,
        right: 0,
        bottom: 0,
        child: Padding(
          // Clear of the screen's notches and bars in fullscreen.
          padding: isFullscreen(context)
              ? MediaQuery.paddingOf(context)
              : EdgeInsets.zero,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Padding(
                padding: const EdgeInsets.only(left: 8, right: 8),
                child: Row(
                  children: [widget.readout, const Spacer(), widget.fullscreen],
                ),
              ),
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16),
                child: widget.bar,
              ),
            ],
          ),
        ),
      ),
    ],
  );
}

/// A double tap's step on its side: `−10 s` or `+10 s`.
class _Flash extends StatelessWidget {
  const new({required this.step});

  final Duration step;

  @override
  Widget build(BuildContext context) {
    final seconds = step.inSeconds.abs();
    return DecoratedBox(
      decoration: BoxDecoration(
        color: _backdrop,
        borderRadius: BorderRadius.circular(20),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
        child: Text(
          step.isNegative ? '−$seconds s' : '+$seconds s',
          style: const TextStyle(color: _white, fontSize: 16),
        ),
      ),
    );
  }
}
