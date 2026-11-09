import 'package:flutter/material.dart';
import 'package:jitter/widgets/refresh_key.dart';

/// One pane of [HomePanes]: a top-level page under its title, and what
/// Ctrl+R refreshes on it (`null`: nothing).
typedef HomePane = ({String title, Widget page, VoidCallback? refresh});

/// Top-level pages side by side, each under its title, for a window wide
/// enough to show them at once. Tab goes through one pane before it enters
/// the next. While [active], Ctrl+R refreshes the pane with the focus, or
/// every pane while none has it.
class HomePanes extends StatefulWidget {
  const new({required this.panes, required this.active, super.key});

  final List<HomePane> panes;

  /// Whether the panes are the page shown, so Ctrl+R is theirs.
  final bool active;

  @override
  State<HomePanes> createState() => _HomePanesState();
}

class _HomePanesState extends State<HomePanes> {
  /// Whether the focus is in each pane, one node per pane.
  final List<FocusNode> _focus = [];

  @override
  void initState() {
    super.initState();
    _fitFocus();
  }

  @override
  void didUpdateWidget(HomePanes oldWidget) {
    super.didUpdateWidget(oldWidget);
    _fitFocus();
  }

  /// One focus node for each pane.
  void _fitFocus() {
    while (_focus.length > widget.panes.length) {
      _focus.removeLast().dispose();
    }
    while (_focus.length < widget.panes.length) {
      _focus.add(FocusNode(debugLabel: 'Home pane ${_focus.length}'));
    }
  }

  @override
  void dispose() {
    for (final node in _focus) {
      node.dispose();
    }
    super.dispose();
  }

  void _refresh() {
    final panes = widget.panes;
    final focused = [
      for (var i = 0; i < panes.length; i++)
        if (_focus[i].hasFocus) panes[i],
    ];
    for (final pane in focused.isEmpty ? panes : focused) {
      pane.refresh?.call();
    }
  }

  @override
  Widget build(BuildContext context) {
    final panes = widget.panes;
    return RefreshKey(
      onRefresh: widget.active ? _refresh : null,
      child: FocusTraversalGroup(
        policy: OrderedTraversalPolicy(),
        child: Row(
          children: [
            for (var i = 0; i < panes.length; i++) ...[
              if (i > 0) const VerticalDivider(width: 1),
              Expanded(
                child: FocusTraversalOrder(
                  order: NumericFocusOrder(i.toDouble()),
                  child: FocusTraversalGroup(
                    child: Focus(
                      focusNode: _focus[i],
                      canRequestFocus: false,
                      skipTraversal: true,
                      includeSemantics: false,
                      child: _Pane(pane: panes[i]),
                    ),
                  ),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

/// [pane]'s page under its title.
class _Pane extends StatelessWidget {
  const new({required this.pane});

  final HomePane pane;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 4),
          child: Semantics(
            header: true,
            child: Text(
              pane.title,
              style: Theme.of(context).textTheme.titleMedium,
            ),
          ),
        ),
        Expanded(child: pane.page),
      ],
    );
  }
}
