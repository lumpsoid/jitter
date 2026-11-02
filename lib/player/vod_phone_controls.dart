import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:jitter/player/scrub.dart';
import 'package:jitter/player/seek_bar.dart';
import 'package:jitter/player/vod_keys.dart';
import 'package:media_kit_video/media_kit_video.dart';

/// How long the controls stay after the last finger lifts.
const _hideAfter = Duration(seconds: 3);

const _fade = Duration(milliseconds: 300);

/// How long a double tap's `−10s` or `+10s` shows, and how long after
/// it, or after each tap of its streak, a lone tap on a side goes on.
const _flashFor = Duration(milliseconds: 700);

/// How far the flash's outer edge sits from its side, as a share of the
/// width: about where a thumb double taps.
const _flashInset = 0.12;

/// The least room kept from a fullscreen's edges, notch or none: a phone's
/// rounded corners cut into its sides and bottom, and a thumb at its very
/// edge reaches the bar badly.
const _fullscreenMargin = EdgeInsets.fromLTRB(32, 0, 32, 16);

const _backdrop = Color(0x66000000);
const _white = Color(0xFFFFFFFF);

/// A VOD's controls on a phone, in place of media_kit's. A tap shows them
/// over the dimmed picture and another hides them, as do 3 s after the
/// last finger lifts. A finger down anywhere on them holds them: a long,
/// slow drag on the [bar] keeps it, where media_kit's hide 3 s after the
/// tap that showed them, drag or not. A double tap on the left or right
/// third goes [vodSeekStep] back or on, shown or not, and flashes it;
/// while the flash shows, each lone tap on a side goes another step at
/// once, the flash adding them up (`+30s`), and a tap on the other side
/// starts its own total.
/// While a finger drags the [bar], its time shows big over the picture,
/// clear of the finger, with how far it has gone and how slowed it is
/// ([ScrubNotification]).
///
/// The parts are given: media_kit's [playPause] and [fullscreen] buttons,
/// the page's [readout], seek [bar] and, for a rewound live stream, its
/// [mode]; these controls only show and hide them.
class VodPhoneControls extends StatefulWidget {
  const new({
    required this.playPause,
    required this.readout,
    required this.fullscreen,
    required this.bar,
    required this.bufferings,
    required this.onSeekBy,
    this.mode,
    super.key,
  });

  final Widget playPause;
  final Widget readout;
  final Widget fullscreen;
  final Widget bar;

  /// After the [readout], away from [fullscreen] so neither is tapped for
  /// the other.
  final Widget? mode;

  /// Whether the player waits for the VOD to load: now, then each change;
  /// a new stream each call. The spinner shows in place of [playPause].
  final Stream<bool> Function() bufferings;

  /// Moves the VOD by a double tap's step, or a streak tap's.
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

  /// How far a double tap and the taps of its streak just went, shown on
  /// its side; null when none, and then no streak goes on.
  Duration? _flash;
  Timer? _flashing;

  /// The bar's drag under way, or null.
  ScrubReading? _scrub;

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

  /// The step a tap at [x] goes: back on the left third, on on the right,
  /// none in the middle.
  static Duration? _stepAt(double x, double width) => width <= 0
      ? null
      : x < width / 3
      ? -vodSeekStep
      : x > width * 2 / 3
      ? vodSeekStep
      : null;

  void _onDoubleTap(double width) {
    final x = _doubleTapAt?.dx;
    if (x == null) return;
    if (_stepAt(x, width) case final step?) _go(step);
  }

  /// A lone tap while the flash shows: a side's goes on at once; the
  /// middle's ends the streak and is a tap as any other.
  void _onStreakTap(double x, double width) {
    if (_stepAt(x, width) case final step?) return _go(step);
    _flashing?.cancel();
    setState(() => _flash = null);
    _visible ? _hideNow() : _show();
  }

  /// Goes [step], added to the flash's total when that went the same way.
  void _go(Duration step) {
    widget.onSeekBy(step);
    _flashing?.cancel();
    final total = _flash;
    setState(
      () => _flash = total != null && total.isNegative == step.isNegative
          ? total + step
          : step,
    );
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
            // In a streak a lone tap goes at once: no double tap to wait
            // for.
            if (flash == null)
              GestureDetector(
                behavior: HitTestBehavior.opaque,
                onTap: () => _visible ? _hideNow() : _show(),
                onDoubleTapDown: (details) =>
                    _doubleTapAt = details.localPosition,
                onDoubleTap: () => _onDoubleTap(constraints.maxWidth),
              )
            else
              GestureDetector(
                behavior: HitTestBehavior.opaque,
                onTapDown: (details) => _onStreakTap(
                  details.localPosition.dx,
                  constraints.maxWidth,
                ),
              ),
            if (_built)
              AnimatedOpacity(
                opacity: _visible ? 1 : 0,
                duration: _fade,
                onEnd: () {
                  if (!_visible) {
                    setState(() {
                      _built = false;
                      _scrub = null;
                    });
                  }
                },
                child: IgnorePointer(
                  ignoring: !_visible,
                  child: NotificationListener<ScrubNotification>(
                    onNotification: (notification) {
                      setState(() => _scrub = notification.reading);
                      return true;
                    },
                    child: _controls(),
                  ),
                ),
              ),
            if (_buffering)
              const IgnorePointer(
                child: Center(child: CircularProgressIndicator(color: _white)),
              ),
            if (flash != null)
              // Held by its outer edge, so a total grown longer
              // (`+50s`, `+1m 00s`) widens it toward the middle and its
              // side's end stays put.
              IgnorePointer(
                child: Align(
                  alignment: flash.isNegative
                      ? Alignment.centerLeft
                      : Alignment.centerRight,
                  child: Padding(
                    padding: EdgeInsets.symmetric(
                      horizontal: constraints.maxWidth * _flashInset,
                    ),
                    child: _Flash(step: flash),
                  ),
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
      if (_scrub case final scrub?)
        // Up, away from the finger on the bar and from the spinner.
        IgnorePointer(
          child: Align(
            alignment: const Alignment(0, -0.6),
            child: _ScrubReadout(scrub),
          ),
        ),
      Positioned(
        left: 0,
        right: 0,
        bottom: 0,
        child: Padding(
          // Clear of the screen's notches, bars and corners in fullscreen.
          padding: isFullscreen(context)
              ? MediaQuery.paddingOf(context).clamp(
                  _fullscreenMargin,
                  const EdgeInsets.all(double.infinity),
                )
              : EdgeInsets.zero,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Padding(
                padding: const EdgeInsets.only(left: 8, right: 8),
                child: Row(
                  children: [
                    widget.readout,
                    ?widget.mode,
                    const Spacer(),
                    widget.fullscreen,
                  ],
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

/// A drag's time, big; under it how far it has gone, and how slowed it is
/// or how to slow it.
class _ScrubReadout extends StatelessWidget {
  const new(this.scrub);

  final ScrubReading scrub;

  @override
  Widget build(BuildContext context) {
    final speed = scrub.speed == ScrubSpeed.normal
        ? 'slide up for finer'
        : scrub.speed.label;
    const figures = [FontFeature.tabularFigures()];
    return DecoratedBox(
      decoration: BoxDecoration(
        color: _backdrop,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 10),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              clock(scrub.target),
              style: const TextStyle(
                color: _white,
                fontSize: 32,
                fontWeight: FontWeight.w500,
                fontFeatures: figures,
              ),
            ),
            Text(
              '${signedClock(scrub.offset)}  ·  $speed',
              style: const TextStyle(
                color: _white,
                fontSize: 14,
                fontFeatures: figures,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// How far a double tap and its streak went, on its side: `−10s`, `+1m 30s`.
class _Flash extends StatelessWidget {
  const new({required this.step});

  final Duration step;

  /// `30s` under a minute, then `1m 00s`, `1m 30s`: from a minute on it
  /// keeps one width as the seconds go round.
  String get _amount {
    final minutes = step.inMinutes.abs();
    final seconds = step.inSeconds.abs() % 60;
    return minutes == 0
        ? '${seconds}s'
        : '${minutes}m ${seconds.toString().padLeft(2, '0')}s';
  }

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: _backdrop,
        borderRadius: BorderRadius.circular(20),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
        child: Text(
          step.isNegative ? '−$_amount' : '+$_amount',
          style: const TextStyle(
            color: _white,
            fontSize: 16,
            fontFeatures: [FontFeature.tabularFigures()],
          ),
        ),
      ),
    );
  }
}
