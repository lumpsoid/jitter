import 'package:flutter/material.dart';
import 'package:jitter/widgets/focus_ring.dart';

/// The black 16:9 box the video plays in, labelled "Video player" for a
/// screen reader and ringed while the focus is anywhere in it.
///
/// It is one Tab stop. While [childTakesFocus] that stop is the child's own
/// (media_kit's video, whose keys — Space, `f`, the arrows — only reach it
/// while it has the focus; or the failure's Retry); otherwise, while the
/// stream loads, the box itself is the stop.
class VideoFrame extends StatefulWidget {
  const new({required this.childTakesFocus, required this.child, super.key});

  final bool childTakesFocus;
  final Widget child;

  @override
  State<VideoFrame> createState() => _VideoFrameState();
}

class _VideoFrameState extends State<VideoFrame> {
  final _focus = FocusNode(debugLabel: 'Video frame');
  bool _focused = false;

  @override
  void didUpdateWidget(VideoFrame oldWidget) {
    super.didUpdateWidget(oldWidget);
    // The box had the focus while loading and the video has come: hand the
    // focus to the video once it is built, so its keys work.
    if (widget.childTakesFocus && !oldWidget.childTakesFocus) {
      WidgetsBinding.instance.addPostFrameCallback((_) => _handToChild());
    }
  }

  void _handToChild() {
    if (mounted && _focus.hasPrimaryFocus) _childStop?.requestFocus();
  }

  /// The child's own Tab stop: the outermost one in the box, so media_kit's
  /// video rather than one of its buttons.
  FocusNode? get _childStop {
    bool isStop(FocusNode node) => node.canRequestFocus && !node.skipTraversal;
    for (final node in _focus.traversalDescendants) {
      final between = node.ancestors.takeWhile((a) => a != _focus);
      if (!between.any(isStop)) return node;
    }
    return null;
  }

  @override
  void dispose() {
    _focus.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Semantics(
      container: true,
      label: 'Video player',
      child: Focus(
        focusNode: _focus,
        skipTraversal: widget.childTakesFocus,
        onFocusChange: (focused) => setState(() => _focused = focused),
        child: FocusRing(
          shown: _focused,
          child: AspectRatio(
            aspectRatio: 16 / 9,
            child: ColoredBox(color: Colors.black, child: widget.child),
          ),
        ),
      ),
    );
  }
}
