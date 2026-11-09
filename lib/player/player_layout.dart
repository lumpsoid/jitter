import 'dart:math' as math;

import 'package:flutter/material.dart';

/// From this width the chat sits beside the video, below it otherwise.
const double _chatBesideMinWidth = 840;

/// The chat's share of the body's width beside the video, held between
/// [_chatMinWidth] and [_chatMaxWidth]: a wide window gets the full width,
/// a narrow one (or a scaled-up display, fewer logical pixels across)
/// keeps more of the video.
const double _chatShare = 0.25;
const double _chatMinWidth = 240;
const double _chatMaxWidth = 340;

/// How wide the chat may grow into the room a height-limited video leaves
/// (an ultrawide window); past it the video sits between bars.
const double _chatWideMaxWidth = 640;

/// The video's shape, as `VideoFrame` holds it.
const double _videoAspect = 16 / 9;

/// The `NowPlaying` strip's height at the default text size: two lines and
/// their padding. Only an estimate of the video's room: the strip keeps its
/// own height, so a larger text size costs a few pixels of bars, not an
/// overflow.
const double _nowPlayingHeight = 52;

/// The chat's width beside the video in a [body] that size, the strip over
/// the video if [nowPlaying]: its share of the width, or the room the
/// video, limited by the height, leaves beside it, if that is more.
double chatWidthBeside(Size body, {required bool nowPlaying}) {
  final share = (body.width * _chatShare).clamp(_chatMinWidth, _chatMaxWidth);
  final videoHeight = body.height - (nowPlaying ? _nowPlayingHeight : 0);
  final leftover = body.width - 1 - videoHeight * _videoAspect;
  return math.max(share, math.min(leftover, _chatWideMaxWidth));
}

/// The player page's body: the [video] alone (no [chat], or hidden), or
/// with the [chat] beside it (wide) or below it (narrow). The [nowPlaying]
/// strip heads the video's column: at the top of the body, over the
/// video, beside the chat rather than over it. Tab goes through the video,
/// then the chat, in either layout: each is its own traversal group, so
/// the focus finishes one before it enters the other.
class PlayerLayout extends StatelessWidget {
  const new({
    required this.video,
    required this.chat,
    this.nowPlaying,
    super.key,
  });

  final Widget video;
  final Widget? chat;
  final Widget? nowPlaying;

  @override
  Widget build(BuildContext context) {
    final video = _group(1, this.video);
    final chat = this.chat;
    final nowPlaying = this.nowPlaying;
    final videoColumn = Column(
      children: [
        ?nowPlaying,
        Expanded(child: Center(child: video)),
      ],
    );
    return FocusTraversalGroup(
      policy: OrderedTraversalPolicy(),
      child: chat == null
          ? videoColumn
          : LayoutBuilder(
              builder: (context, constraints) {
                if (constraints.maxWidth >= _chatBesideMinWidth) {
                  final chatWidth = chatWidthBeside(
                    constraints.biggest,
                    nowPlaying: nowPlaying != null,
                  );
                  return Row(
                    children: [
                      Expanded(child: videoColumn),
                      const VerticalDivider(width: 1),
                      SizedBox(width: chatWidth, child: _group(2, chat)),
                    ],
                  );
                }
                return Column(
                  children: [
                    ?nowPlaying,
                    video,
                    const Divider(height: 1),
                    Expanded(child: _group(2, chat)),
                  ],
                );
              },
            ),
    );
  }

  /// [child] as one traversal group, [order]th on the page's body.
  Widget _group(double order, Widget child) => FocusTraversalOrder(
    order: NumericFocusOrder(order),
    child: FocusTraversalGroup(child: child),
  );
}
