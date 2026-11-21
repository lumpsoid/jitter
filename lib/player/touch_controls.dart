import 'dart:async';

import 'package:jitter/player/controls_fade.dart';
import 'package:material_ui/material_ui.dart';

/// How long the controls stay after the last tap that showed them.
const _hideAfter = Duration(seconds: 3);

/// How long the controls and the buffering spinner take to fade.
const _fade = Duration(milliseconds: 300);

/// The height of the top and bottom bars, the strips the buttons sit in.
const _barHeight = 56.0;

const _clear = Color(0x00000000);
const _white = Color(0xFFFFFFFF);

/// Dims the video behind the controls.
const _scrim = Color(0x66000000);

/// The touch controls over a live player's video, in place of media_kit's
/// `MaterialVideoControls`. A tap on the video shows the controls, which hide
/// [_hideAfter] after that tap, or at once on a second tap. [center] sits in
/// the middle of the video, and [bottomBar] lays out in a row along the
/// bottom, inside [padding]. A tap on a control presses it and does not
/// toggle the controls.
///
/// The [bufferings] spinner shows over the video even while the controls are
/// hidden, and does not take a tap.
class TouchControls extends StatefulWidget {
  const new({
    required this.center,
    required this.bottomBar,
    required this.bufferings,
    this.padding = EdgeInsets.zero,
    super.key,
  });

  final Widget center;
  final List<Widget> bottomBar;

  /// Whether the player waits for the stream to load: now, then each
  /// change; a new stream each call.
  final Stream<bool> Function() bufferings;

  /// Keeps the controls clear of the video's edges.
  final EdgeInsets padding;

  @override
  State<TouchControls> createState() => _TouchControlsState();
}

class _TouchControlsState extends State<TouchControls>
    with SingleTickerProviderStateMixin {
  /// Whether the controls are shown, or fading in or out.
  bool _visible = false;

  /// Whether the controls are built: through the fade out too, then not.
  bool _built = false;

  late final AnimationController _controller;

  /// The fade the controls' colours follow: 0 hidden, 1 shown.
  late final CurvedAnimation _fadeCurve;

  Timer? _hide;

  bool _buffering = false;
  late final StreamSubscription<bool> _bufferings;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(vsync: this, duration: _fade)
      ..addStatusListener((status) {
        if (status == AnimationStatus.dismissed) {
          setState(() => _built = false);
        }
      });
    _fadeCurve = CurvedAnimation(parent: _controller, curve: Curves.easeInOut);
    _bufferings = widget.bufferings().listen(
      (buffering) => setState(() => _buffering = buffering),
    );
  }

  @override
  void dispose() {
    _hide?.cancel();
    unawaited(_bufferings.cancel());
    _fadeCurve.dispose();
    _controller.dispose();
    super.dispose();
  }

  void _onTap() {
    if (_visible) {
      _hideNow();
    } else {
      _show();
    }
  }

  /// Shows the controls, and hides them [_hideAfter] after this tap.
  void _show() {
    setState(() {
      _visible = true;
      _built = true;
    });
    _controller.forward();
    _hide = Timer(_hideAfter, _hideNow);
  }

  void _hideNow() {
    _hide?.cancel();
    if (_visible) {
      setState(() => _visible = false);
      _controller.reverse();
    }
  }

  @override
  Widget build(BuildContext context) {
    return Theme(
      data: Theme.of(context).copyWith(
        focusColor: _clear,
        hoverColor: _clear,
        splashColor: _clear,
        highlightColor: _clear,
      ),
      child: Focus(
        autofocus: true,
        child: Material(
          color: _clear,
          child: Stack(
            fit: StackFit.expand,
            children: [
              // Below the controls, so a tap on one of them goes to it.
              Positioned.fill(
                left: 16,
                top: 16,
                right: 16,
                bottom: 16,
                child: Listener(
                  behavior: HitTestBehavior.opaque,
                  onPointerDown: (_) => _onTap(),
                ),
              ),
              ControlsFade(
                notifier: _fadeCurve,
                child: _built
                    ? IgnorePointer(ignoring: !_visible, child: _controls())
                    : const SizedBox.shrink(),
              ),
              IgnorePointer(
                child: Padding(
                  padding: widget.padding,
                  child: Column(
                    children: [
                      const SizedBox(height: _barHeight),
                      Expanded(
                        child: Center(
                          child: TweenAnimationBuilder<double>(
                            tween: Tween<double>(
                              begin: 0,
                              end: _buffering ? 1 : 0,
                            ),
                            duration: _fade,
                            builder: (context, value, _) => value > 0
                                ? CircularProgressIndicator(
                                    color: _white.withValues(alpha: value),
                                  )
                                : const SizedBox.shrink(),
                          ),
                        ),
                      ),
                      const SizedBox(height: _barHeight),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  /// The dimmed video with the centre button and the bottom bar over it.
  Widget _controls() => Stack(
    fit: StackFit.expand,
    children: [
      // Decoration only: a tap on the dimmed video must reach the listener.
      const IgnorePointer(child: _Scrim()),
      Padding(
        padding: widget.padding,
        child: Column(
          children: [
            const SizedBox(height: _barHeight),
            Expanded(
              // Gone while buffering, not faded: it takes no press then.
              child: _buffering
                  ? const SizedBox.shrink()
                  : Center(child: widget.center),
            ),
            Container(
              height: _barHeight,
              margin: const EdgeInsets.only(left: 16, right: 8),
              child: Row(children: widget.bottomBar),
            ),
          ],
        ),
      ),
    ],
  );
}

/// Dims the video behind the controls, fading with them.
class _Scrim extends StatelessWidget {
  const new();

  @override
  Widget build(BuildContext context) =>
      ColoredBox(color: fadedColor(context, _scrim));
}
