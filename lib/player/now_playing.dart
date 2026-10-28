import 'package:flutter/material.dart';
import 'package:jitter/player/chapter_button.dart';
import 'package:jitter/src/bindings/bindings.dart';

/// The strip over the player page's video: the stream's or the VOD's
/// [title], cut to a line with the whole of it on hover or long press, and
/// the [game] being played under it. For a VOD with [chapters] the whole
/// strip is a button to them ([ChapterButton]), a drop-down arrow beside
/// the game, and calls [onGoToChapter] with the one picked. Each line
/// keeps its room while its text is unknown (loading, or none to show),
/// so the video under the strip stays put when they arrive.
class NowPlaying extends StatelessWidget {
  const new({
    required this.onGoToChapter,
    this.title,
    this.game,
    this.chapters = const [],
    super.key,
  });

  final String? title;
  final String? game;
  final List<PlaybackChapter> chapters;
  final ValueChanged<int> onGoToChapter;

  @override
  Widget build(BuildContext context) {
    final title = this.title;
    final theme = Theme.of(context);
    final titleLine = _line(title, theme.textTheme.titleSmall);
    final gameStyle = theme.textTheme.bodySmall?.copyWith(
      color: theme.colorScheme.primary,
    );
    final gameLine = _line(game, gameStyle);
    final strip = Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        mainAxisSize: MainAxisSize.min,
        children: [
          if (title == null)
            titleLine
          else
            Tooltip(message: title, child: titleLine),
          if (chapters.isEmpty)
            gameLine
          else
            Row(
              children: [
                Flexible(child: gameLine),
                // As high as the game's line, so the strip does not grow.
                Icon(
                  Icons.arrow_drop_down,
                  size: _lineHeight(gameStyle),
                  color: gameStyle?.color,
                ),
              ],
            ),
        ],
      ),
    );
    if (chapters.isEmpty) return strip;
    return ChapterButton(
      chapters: chapters,
      onGoTo: onGoToChapter,
      child: strip,
    );
  }

  /// One line of [text]; an empty one, a line high, while it is unknown.
  Text _line(String? text, TextStyle? style) => Text(
    text ?? '',
    maxLines: 1,
    overflow: TextOverflow.ellipsis,
    style: style,
  );

  double _lineHeight(TextStyle? style) =>
      (style?.fontSize ?? 14) * (style?.height ?? 1);
}
