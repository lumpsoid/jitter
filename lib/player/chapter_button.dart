import 'dart:async';

import 'package:flutter/material.dart';
import 'package:jitter/src/bindings/bindings.dart';

/// Below this width the chapters open as a bottom sheet, a menu otherwise,
/// as the player's options do.
const double _sheetMaxWidth = 600;

/// [child] as a button to a VOD's [chapters]: each with its game and
/// start, the one playing checked. Picking one calls [onGoTo] with its
/// index. The whole of [child] takes the tap, so a strip of small text is
/// still a target a thumb can hit.
class ChapterButton extends StatelessWidget {
  const new({
    required this.chapters,
    required this.onGoTo,
    required this.child,
    super.key,
  });

  final List<PlaybackChapter> chapters;
  final ValueChanged<int> onGoTo;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    if (MediaQuery.sizeOf(context).width < _sheetMaxWidth) {
      return _button(() => _showSheet(context));
    }
    return MenuAnchor(
      menuChildren: [
        for (final (index, chapter) in chapters.indexed)
          MenuItemButton(
            leadingIcon: _check(checked: chapter.current),
            trailingIcon: Text(chapter.start),
            onPressed: () => onGoTo(index),
            child: Text(chapter.game),
          ),
      ],
      builder: (context, controller, _) => _button(
        () => controller.isOpen ? controller.close() : controller.open(),
      ),
    );
  }

  Widget _button(VoidCallback onTap) => Semantics(
    button: true,
    label: 'Chapters',
    child: InkWell(onTap: onTap, child: child),
  );

  void _showSheet(BuildContext context) => unawaited(
    showModalBottomSheet<void>(
      context: context,
      showDragHandle: true,
      builder: (sheet) => SafeArea(
        child: ListView(
          shrinkWrap: true,
          children: [
            for (final (index, chapter) in chapters.indexed)
              ListTile(
                leading: _check(checked: chapter.current),
                title: Text(chapter.game),
                trailing: Text(chapter.start),
                onTap: () {
                  Navigator.pop(sheet);
                  onGoTo(index);
                },
              ),
          ],
        ),
      ),
    ),
  );
}

/// A check before the chapter playing, the same space before the others.
Widget _check({required bool checked}) =>
    checked ? const Icon(Icons.check) : const SizedBox.square(dimension: 24);
