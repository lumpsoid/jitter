import 'package:flutter/material.dart';

/// Lines of the description shown while it is collapsed.
const int _collapsedLines = 3;

/// Tallest the expanded description grows before it scrolls, so the VODs
/// under it keep their room.
const double _expandedMaxHeight = 240;

/// A channel's description, collapsed to a few lines. When it does not fit
/// them, "Show more" expands it and "Show less" collapses it again.
class ChannelDescription extends StatefulWidget {
  const new({required this.text, super.key});

  final String text;

  @override
  State<ChannelDescription> createState() => _ChannelDescriptionState();
}

class _ChannelDescriptionState extends State<ChannelDescription> {
  bool _expanded = false;

  @override
  void didUpdateWidget(ChannelDescription oldWidget) {
    super.didUpdateWidget(oldWidget);
    // Another channel's description starts collapsed.
    if (oldWidget.text != widget.text) _expanded = false;
  }

  @override
  Widget build(BuildContext context) {
    final style = Theme.of(context).textTheme.bodyMedium;
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 0, 16, 8),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final painter = TextPainter(
            text: TextSpan(text: widget.text, style: style),
            maxLines: _collapsedLines,
            textDirection: Directionality.of(context),
            textScaler: MediaQuery.textScalerOf(context),
          )..layout(maxWidth: constraints.maxWidth);
          final overflows = painter.didExceedMaxLines;
          painter.dispose();
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              if (_expanded)
                ConstrainedBox(
                  constraints: const BoxConstraints(
                    maxHeight: _expandedMaxHeight,
                  ),
                  child: SingleChildScrollView(
                    child: Text(widget.text, style: style),
                  ),
                )
              else
                Text(
                  widget.text,
                  style: style,
                  maxLines: _collapsedLines,
                  overflow: TextOverflow.ellipsis,
                ),
              if (overflows)
                TextButton(
                  onPressed: () => setState(() => _expanded = !_expanded),
                  child: Text(_expanded ? 'Show less' : 'Show more'),
                ),
            ],
          );
        },
      ),
    );
  }
}
