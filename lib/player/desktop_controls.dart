import 'dart:async';

import 'package:material_ui/material_ui.dart';

/// How long the controls stay after the last mouse move over the video.
const _hideAfter = Duration(seconds: 3);

/// How long the controls and the buffering spinner take to fade.
const _fade = Duration(milliseconds: 150);

/// How soon after a click a second one counts as a double click.
const _doubleClick = Duration(milliseconds: 400);

const _clear = Color(0x00000000);
const _white = Color(0xFFFFFFFF);

/// Darkens the bottom of the video behind the bar.
const _shade = LinearGradient(
  begin: Alignment.topCenter,
  end: Alignment.bottomCenter,
  stops: [0.5, 1.0],
  colors: [_clear, Color(0x61000000)],
);

/// The desktop controls over a player's video, in place of media_kit's
/// `MaterialDesktopVideoControls`. A mouse move over the video shows the
/// bottom bar, which hides [_hideAfter] after the last move, or at once
/// when the mouse leaves the video. A double click on the video calls
/// [onToggleFullscreen]. The bar is given: [bottomBar] lays out in a row
/// in a [barHeight] strip, inside [padding]. [keys] are the shortcuts,
/// held while the video has the focus.
///
/// The [bufferings] spinner shows over the video even while the bar is
/// hidden, and does not take a click.
class DesktopControls extends StatefulWidget {
  const new({
    required this.keys,
    required this.bottomBar,
    required this.bufferings,
    required this.onToggleFullscreen,
    this.barHeight = 56,
    this.padding = EdgeInsets.zero,
    this.focusNode,
    super.key,
  });

  final Map<ShortcutActivator, VoidCallback> keys;
  final List<Widget> bottomBar;

  /// Whether the player waits for the stream to load: now, then each
  /// change; a new stream each call.
  final Stream<bool> Function() bufferings;

  final VoidCallback onToggleFullscreen;

  final double barHeight;

  /// Keeps the bar clear of the video's edges.
  final EdgeInsets padding;

  /// Lets the page put the keyboard on the controls: the focus the keys are
  /// held by. Null, the controls take the focus on their own at first.
  final FocusNode? focusNode;

  @override
  State<DesktopControls> createState() => _DesktopControlsState();
}

class _DesktopControlsState extends State<DesktopControls> {
  /// Whether the bar is shown, or fading in or out.
  bool _visible = false;

  /// Whether the bar is built: through the fade out too, then not.
  bool _built = false;

  Timer? _hide;

  /// Running after a click, the time a second one may come in; null when
  /// none is awaited, so a click after a double click starts a new pair.
  Timer? _clickWindow;

  bool _buffering = false;
  late final StreamSubscription<bool> _bufferings;

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
    _clickWindow?.cancel();
    unawaited(_bufferings.cancel());
    super.dispose();
  }

  /// Shows the bar, and hides it [_hideAfter] after the last call.
  void _show() {
    _hide?.cancel();
    if (!_visible) {
      setState(() {
        _visible = true;
        _built = true;
      });
    }
    _hide = Timer(_hideAfter, _hideNow);
  }

  void _hideNow() {
    _hide?.cancel();
    if (_visible) setState(() => _visible = false);
  }

  void _onTapUp(TapUpDetails _) {
    if (_clickWindow case final window?) {
      window.cancel();
      _clickWindow = null;
      widget.onToggleFullscreen();
    } else {
      _clickWindow = Timer(_doubleClick, () => _clickWindow = null);
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
      child: CallbackShortcuts(
        bindings: widget.keys,
        child: Focus(
          focusNode: widget.focusNode,
          autofocus: true,
          child: Material(
            color: _clear,
            child: GestureDetector(
              behavior: HitTestBehavior.opaque,
              onTapUp: _onTapUp,
              child: MouseRegion(
                onEnter: (_) => _show(),
                onHover: (_) => _show(),
                onExit: (_) => _hideNow(),
                child: Stack(
                  fit: StackFit.expand,
                  children: [
                    AnimatedOpacity(
                      opacity: _visible ? 1 : 0,
                      duration: _fade,
                      curve: Curves.easeInOut,
                      onEnd: () {
                        if (!_visible) setState(() => _built = false);
                      },
                      child: _built
                          ? IgnorePointer(
                              ignoring: !_visible,
                              child: Stack(
                                fit: StackFit.expand,
                                children: [
                                  const DecoratedBox(
                                    decoration: BoxDecoration(gradient: _shade),
                                  ),
                                  _bar(),
                                ],
                              ),
                            )
                          : const SizedBox.shrink(),
                    ),
                    IgnorePointer(
                      child: Padding(
                        padding: widget.padding,
                        child: Column(
                          children: [
                            SizedBox(height: widget.barHeight),
                            Expanded(
                              child: Center(
                                child: TweenAnimationBuilder<double>(
                                  tween: Tween<double>(
                                    begin: 0,
                                    end: _buffering ? 1 : 0,
                                  ),
                                  duration: _fade,
                                  builder: (context, value, _) => value > 0
                                      ? Opacity(
                                          opacity: value,
                                          child:
                                              const CircularProgressIndicator(
                                                color: _white,
                                              ),
                                        )
                                      : const SizedBox.shrink(),
                                ),
                              ),
                            ),
                            SizedBox(height: widget.barHeight),
                          ],
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _bar() => Padding(
    padding: widget.padding,
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.end,
      children: [
        const Spacer(),
        Container(
          height: widget.barHeight,
          margin: const EdgeInsets.symmetric(horizontal: 16),
          child: Row(children: widget.bottomBar),
        ),
      ],
    ),
  );
}
