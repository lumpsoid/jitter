import 'package:flutter/material.dart';

/// The strip over the player page's video: the stream's or the VOD's
/// [title], cut to a line with the whole of it on hover or long press, and
/// the [game] being played under it. Each line keeps its room while its
/// text is unknown (loading, or none to show), so the video under the
/// strip stays put when they arrive.
class NowPlaying extends StatelessWidget {
  const new({this.title, this.game, super.key});

  final String? title;
  final String? game;

  @override
  Widget build(BuildContext context) {
    final title = this.title;
    final theme = Theme.of(context);
    final titleLine = _line(title, theme.textTheme.titleSmall);
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        mainAxisSize: MainAxisSize.min,
        children: [
          if (title == null)
            titleLine
          else
            Tooltip(message: title, child: titleLine),
          _line(
            game,
            theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.primary,
            ),
          ),
        ],
      ),
    );
  }

  /// One line of [text]; an empty one, a line high, while it is unknown.
  Text _line(String? text, TextStyle? style) => Text(
    text ?? '',
    maxLines: 1,
    overflow: TextOverflow.ellipsis,
    style: style,
  );
}
