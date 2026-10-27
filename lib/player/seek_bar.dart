import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:jitter/player/scrub.dart';

/// The bar's height, the pointer's target; the track is drawn in its
/// middle, thinner.
const double seekBarHeight = 36;

const double _track = 3.2;
const double _hoveredTrack = 5.6;
const double _thumb = 12;
const _trackColor = Color(0x3DFFFFFF);
const _playedColor = Color(0xFFFF0000);

/// How long a drag must rest on a spot before the player seeks there.
/// Each seek off the buffered part loads a few seconds of the VOD, so a
/// sweep across the bar seeks only where it stops, not at every pixel; a
/// drag that keeps still this long shows its frame.
const _rest = Duration(milliseconds: 150);

/// A VOD's seek bar, in media_kit's desktop controls in place of their own
/// and drawn like it. A click seeks to the spot and a drag follows the
/// pointer; held Shift slows the drag to a tenth and Shift and Ctrl to a
/// hundredth ([ScrubSpeed]), for a precise spot in a long VOD. A slowed
/// press does not jump: the drag starts from where it plays. Over the bar
/// a bubble shows the time under the pointer, or the drag's and its speed.
///
/// A drag seeks the player itself, where it rests and where it ends, so
/// its frame shows. Twitch also publishes preview images along each VOD
/// (`seekPreviewsURL` on a GQL `Video`: a storyboard of sprite sheets); a
/// bubble showing them could let a drag at the normal speed seek only on
/// release, the slowed ones still seeking within what has buffered.
class SeekBar extends StatefulWidget {
  const new({
    required this.positions,
    required this.durations,
    required this.buffers,
    required this.onSeek,
    super.key,
  });

  /// Where it plays, how long the VOD is and how far it has buffered:
  /// each now, then each change. Each call is a new stream: media_kit's
  /// controls keep this one widget and mount it anew each time they show,
  /// and each mount listens from the start.
  final Stream<Duration> Function() positions;
  final Stream<Duration> Function() durations;
  final Stream<Duration> Function() buffers;

  /// Seeks to a click's spot at once, and to a drag's where it rests and
  /// where it ends.
  final ValueChanged<Duration> onSeek;

  @override
  State<SeekBar> createState() => _SeekBarState();
}

class _SeekBarState extends State<SeekBar> {
  Duration _position = Duration.zero;
  Duration _duration = Duration.zero;
  Duration _buffer = Duration.zero;
  late final List<StreamSubscription<Duration>> _subscriptions;

  /// Where over the bar the pointer hovers, or null when off it.
  double? _hoverX;

  /// The drag under way, or null.
  Scrub? _scrub;

  /// Seeks to where the drag rests, until it moves on.
  Timer? _resting;

  /// Where the bar last seeked to, so a drag ending where it rested does
  /// not seek there again.
  Duration? _sought;

  @override
  void initState() {
    super.initState();
    _subscriptions = [
      widget.positions().listen((at) => setState(() => _position = at)),
      widget.durations().listen((at) => setState(() => _duration = at)),
      widget.buffers().listen((at) => setState(() => _buffer = at)),
    ];
    HardwareKeyboard.instance.addHandler(_onKey);
  }

  @override
  void dispose() {
    HardwareKeyboard.instance.removeHandler(_onKey);
    _resting?.cancel();
    for (final subscription in _subscriptions) {
      unawaited(subscription.cancel());
    }
    super.dispose();
  }

  ScrubSpeed get _speed => ScrubSpeed.held(
    shift: HardwareKeyboard.instance.isShiftPressed,
    control: HardwareKeyboard.instance.isControlPressed,
  );

  /// Redraws a drag's speed as Shift or Ctrl goes down or up; the key
  /// goes on to whatever else handles it.
  bool _onKey(KeyEvent event) {
    if (_scrub != null) setState(() {});
    return false;
  }

  double _fraction(Duration at) => _duration > Duration.zero
      ? (at.inMicroseconds / _duration.inMicroseconds).clamp(0.0, 1.0)
      : 0;

  void _start(double x, double width) {
    final scrub = Scrub.start(
      x: x,
      width: width,
      duration: _duration,
      position: _position,
      speed: _speed,
    );
    setState(() => _scrub = scrub);
    _seek(scrub.target);
  }

  void _seek(Duration to) {
    _resting?.cancel();
    _sought = to;
    widget.onSeek(to);
  }

  void _move(DragUpdateDetails details) {
    final scrub = _scrub?.moveTo(details.localPosition.dx, _speed);
    if (scrub == null) return;
    _hoverUnder(details.globalPosition);
    if (scrub.target == _scrub?.target) return;
    setState(() => _scrub = scrub);
    _resting?.cancel();
    _resting = Timer(_rest, () => _seek(scrub.target));
  }

  /// Seeks to where the drag ends, unless it rested there already. Holds
  /// the spot until the player reports from there, so the thumb does not
  /// flick back to where it played before.
  void _end() {
    final scrub = _scrub;
    if (scrub == null) return;
    _resting?.cancel();
    if (scrub.target != _sought) _seek(scrub.target);
    setState(() {
      _position = scrub.target;
      _scrub = null;
    });
  }

  /// media_kit's controls hide a while after the pointer last hovered over
  /// them, and a pressed pointer does not hover: a long, slow drag would
  /// hide them, this bar with them. Each move of the drag is passed to
  /// what is under the pointer as a hover too, which keeps them shown.
  void _hoverUnder(Offset global) {
    final view = View.of(context).viewId;
    final result = HitTestResult();
    WidgetsBinding.instance.hitTestInView(result, global, view);
    final hover = PointerHoverEvent(position: global, viewId: view);
    for (final entry in result.path) {
      entry.target.handleEvent(hover.transformed(entry.transform), entry);
    }
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final width = constraints.maxWidth;
        final scrub = _scrub;
        final hoverX = _hoverX;
        final hovered = hoverX == null || width <= 0
            ? null
            : (hoverX / width).clamp(0.0, 1.0);
        final bubble = scrub != null
            ? _Bubble(
                left: scrub.fraction * width,
                time: scrub.target,
                speed: _speed.label,
              )
            : hovered != null
            ? _Bubble(left: hovered * width, time: _duration * hovered)
            : null;
        return MouseRegion(
          cursor: SystemMouseCursors.click,
          onHover: (event) => setState(() => _hoverX = event.localPosition.dx),
          onExit: (_) => setState(() => _hoverX = null),
          child: GestureDetector(
            behavior: HitTestBehavior.opaque,
            onPanDown: (details) => _start(details.localPosition.dx, width),
            onPanUpdate: _move,
            onPanEnd: (_) => _end(),
            onPanCancel: _end,
            child: SizedBox(
              height: seekBarHeight,
              child: Stack(
                clipBehavior: Clip.none,
                children: [
                  Positioned.fill(
                    child: CustomPaint(
                      painter: _BarPainter(
                        played: scrub?.fraction ?? _fraction(_position),
                        buffered: _fraction(_buffer),
                        hovered: hovered,
                        active: hovered != null || scrub != null,
                      ),
                    ),
                  ),
                  ?bubble,
                ],
              ),
            ),
          ),
        );
      },
    );
  }
}

/// The time over the bar at [left], and a slowed drag's [speed].
class _Bubble extends StatelessWidget {
  const new({required this.left, required this.time, this.speed = ''});

  final double left;
  final Duration time;
  final String speed;

  @override
  Widget build(BuildContext context) {
    return Positioned(
      left: left,
      bottom: seekBarHeight - 4,
      child: IgnorePointer(
        child: FractionalTranslation(
          translation: const Offset(-0.5, 0),
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: const Color(0xCC000000),
              borderRadius: BorderRadius.circular(4),
            ),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 3),
              child: Text(
                speed.isEmpty ? clock(time) : '${clock(time)}  $speed',
                style: const TextStyle(
                  color: Color(0xFFFFFFFF),
                  fontSize: 12,
                  fontFeatures: [FontFeature.tabularFigures()],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// [at] as `h:mm:ss`, or `m:ss` under an hour.
String clock(Duration at) {
  String two(int n) => n.toString().padLeft(2, '0');
  final h = at.inHours;
  final m = at.inMinutes % 60;
  final s = at.inSeconds % 60;
  return h > 0 ? '$h:${two(m)}:${two(s)}' : '$m:${two(s)}';
}

/// The track, filled to [buffered] and [played] (fractions of it), and
/// while [active] thicker, with the thumb and the hovered spot.
class _BarPainter extends CustomPainter {
  const new({
    required this.played,
    required this.buffered,
    required this.hovered,
    required this.active,
  });

  final double played;
  final double buffered;
  final double? hovered;
  final bool active;

  @override
  void paint(Canvas canvas, Size size) {
    final height = active ? _hoveredTrack : _track;
    final top = (size.height - height) / 2;
    Rect upTo(double fraction) =>
        Rect.fromLTWH(0, top, size.width * fraction, height);
    final paint = Paint()..color = _trackColor;
    canvas
      ..drawRect(upTo(1), paint)
      ..drawRect(upTo(buffered), paint);
    if (hovered case final hovered?) canvas.drawRect(upTo(hovered), paint);
    canvas.drawRect(upTo(played), paint..color = _playedColor);
    if (active) {
      canvas.drawCircle(
        Offset(size.width * played, size.height / 2),
        _thumb / 2,
        paint,
      );
    }
  }

  @override
  bool shouldRepaint(_BarPainter old) =>
      old.played != played ||
      old.buffered != buffered ||
      old.hovered != hovered ||
      old.active != active;
}
