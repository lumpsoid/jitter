import 'package:flutter/material.dart';

/// From this width the chat sits beside the video, below it otherwise.
const double _chatBesideMinWidth = 840;

/// The chat's width beside the video.
const double _chatWidth = 340;

/// The player page's body: the [video] alone (no [chat], or hidden), or
/// with the [chat] beside it (wide) or below it (narrow). Tab goes through
/// the video, then the chat, in either layout: each is its own traversal
/// group, so the focus finishes one before it enters the other.
class PlayerLayout extends StatelessWidget {
  const new({required this.video, required this.chat, super.key});

  final Widget video;
  final Widget? chat;

  @override
  Widget build(BuildContext context) {
    final video = _group(1, this.video);
    final chat = this.chat;
    return FocusTraversalGroup(
      policy: OrderedTraversalPolicy(),
      child: chat == null
          ? Center(child: video)
          : LayoutBuilder(
              builder: (context, constraints) {
                if (constraints.maxWidth >= _chatBesideMinWidth) {
                  return Row(
                    children: [
                      Expanded(child: Center(child: video)),
                      const VerticalDivider(width: 1),
                      SizedBox(width: _chatWidth, child: _group(2, chat)),
                    ],
                  );
                }
                return Column(
                  children: [
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
