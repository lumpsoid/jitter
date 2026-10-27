import 'dart:async';

import 'package:flutter/material.dart';
import 'package:jitter/player/go_to_time_dialog.dart';
import 'package:jitter/player/live.dart';
import 'package:jitter/player/seek_bar.dart';
import 'package:jitter/src/bindings/bindings.dart';

/// Where a VOD plays and how long it is (`1:23:45 / 6:00:00`), in
/// media_kit's controls in place of their position indicator. A tap or a
/// click opens a [GoToTimeDialog] to type an exact time to go to.
class PositionReadout extends StatefulWidget {
  const new({
    required this.positions,
    required this.durations,
    required this.answers,
    required this.onGoTo,
    required this.fontSize,
    super.key,
  });

  /// Read now and listened to at each mount, as [SeekBar]'s: the controls
  /// mount this anew each time they show.
  final Live<Duration> positions;
  final Live<Duration> durations;

  /// Rust's answers to [onGoTo], for the dialog.
  final Stream<GoToTimeAnswer> Function() answers;

  /// Sends a typed time.
  final ValueChanged<String> onGoTo;

  /// media_kit's own: smaller on desktop than on a phone.
  final double fontSize;

  @override
  State<PositionReadout> createState() => _PositionReadoutState();
}

class _PositionReadoutState extends State<PositionReadout> {
  late Duration _position = widget.positions.now();
  late Duration _duration = widget.durations.now();
  late final List<StreamSubscription<Duration>> _subscriptions;

  @override
  void initState() {
    super.initState();
    _subscriptions = [
      widget.positions.changes().listen((at) => setState(() => _position = at)),
      widget.durations.changes().listen((at) => setState(() => _duration = at)),
    ];
  }

  @override
  void dispose() {
    for (final subscription in _subscriptions) {
      unawaited(subscription.cancel());
    }
    super.dispose();
  }

  void _open() => unawaited(
    showDialog<void>(
      context: context,
      builder: (_) =>
          GoToTimeDialog(answers: widget.answers(), onSubmit: widget.onGoTo),
    ),
  );

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: 'Go to a time',
      child: MouseRegion(
        cursor: SystemMouseCursors.click,
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTap: _open,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 12),
            child: Text(
              '${clock(_position)} / ${clock(_duration)}',
              style: TextStyle(
                height: 1,
                fontSize: widget.fontSize,
                color: const Color(0xFFFFFFFF),
                fontFeatures: const [FontFeature.tabularFigures()],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
