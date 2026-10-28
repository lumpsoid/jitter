import 'dart:async';

import 'package:flutter/material.dart';
import 'package:jitter/chat/chat_holder.dart';
import 'package:jitter/chat/chat_mode.dart';
import 'package:jitter/player/playback_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';

/// Below this width the options open as a bottom sheet, a menu otherwise.
const double _sheetMaxWidth = 600;

/// The player page's one app bar action: quality and the sleep timer
/// while playback is ready, the chat while there is one, and theatre.
/// A bottom sheet on a narrow screen, a menu with submenus on a wide one.
/// A dot on the button while a sleep timer is armed.
class PlayerOptions extends StatelessWidget {
  const new({
    required this.playback,
    required this.chat,
    required this.onSelectQuality,
    required this.onArmSleep,
    required this.onTurnOffSleep,
    required this.onToggleTheatre,
    super.key,
  });

  final PlaybackHolder playback;
  final ChatHolder chat;
  final ValueChanged<String> onSelectQuality;

  /// Arms a timer of a preset's minutes.
  final ValueChanged<int> onArmSleep;
  final VoidCallback onTurnOffSleep;
  final VoidCallback onToggleTheatre;

  @override
  Widget build(BuildContext context) {
    final left = switch (playback.state) {
      PlaybackUpdateReady(:final sleep) => sleep.left,
      _ => null,
    };
    final icon = Badge(
      isLabelVisible: left != null,
      child: const Icon(Icons.more_vert),
    );
    final tooltip = left == null ? 'Options' : 'Options (sleep timer: $left)';
    if (MediaQuery.sizeOf(context).width < _sheetMaxWidth) {
      return IconButton(
        icon: icon,
        tooltip: tooltip,
        onPressed: () => _showSheet(context),
      );
    }
    return MenuAnchor(
      menuChildren: _menuChildren(),
      builder: (context, controller, _) => IconButton(
        icon: icon,
        tooltip: tooltip,
        onPressed: () =>
            controller.isOpen ? controller.close() : controller.open(),
      ),
    );
  }

  List<Widget> _menuChildren() => [
    if (playback.state case PlaybackUpdateReady(
      :final qualities,
      :final selected,
      :final sleep,
    )) ...[
      SubmenuButton(
        leadingIcon: const Icon(Icons.high_quality_outlined),
        menuChildren: [
          for (final q in qualities)
            MenuItemButton(
              leadingIcon: _check(checked: q.name == selected),
              onPressed: () => onSelectQuality(q.name),
              child: Text(q.label),
            ),
        ],
        child: const Text('Quality'),
      ),
      SubmenuButton(
        leadingIcon: Icon(
          sleep.left == null ? Icons.bedtime_outlined : Icons.bedtime,
        ),
        menuChildren: [
          if (sleep.left case final left?) ...[
            MenuItemButton(child: Text(left)),
            const Divider(height: 1),
          ],
          for (final preset in sleep.presets)
            MenuItemButton(
              onPressed: () => onArmSleep(preset.minutes),
              child: Text(preset.label),
            ),
          if (sleep.left != null) ...[
            const Divider(height: 1),
            MenuItemButton(
              onPressed: onTurnOffSleep,
              child: const Text('Turn off timer'),
            ),
          ],
        ],
        child: const Text('Sleep timer'),
      ),
    ],
    if (chat.mode != ChatMode.off)
      // Not a CheckboxMenuButton: its box sits out of line with the
      // other items' icons.
      MenuItemButton(
        leadingIcon: const Icon(Icons.chat_bubble_outline),
        trailingIcon: chat.mode == ChatMode.shown
            ? const Icon(Icons.check)
            : null,
        onPressed: chat.toggle,
        child: const Text('Chat'),
      ),
    MenuItemButton(
      leadingIcon: const Icon(Icons.fit_screen_outlined),
      onPressed: onToggleTheatre,
      child: const Text('Theatre mode'),
    ),
  ];

  void _showSheet(BuildContext context) => unawaited(
    showModalBottomSheet<void>(
      context: context,
      showDragHandle: true,
      builder: (_) => _OptionsSheet(options: this),
    ),
  );
}

/// A check before the selected item, the same space before the others.
Widget _check({required bool checked}) =>
    checked ? const Icon(Icons.check) : const SizedBox.square(dimension: 24);

/// Which list the sheet shows.
enum _SheetPage { main, quality, sleep }

/// The options as a bottom sheet: the main list, or quality's or the
/// sleep timer's list in its place with a way back. Follows the holders
/// while open. A choice closes it; the chat switch does not.
class _OptionsSheet extends StatefulWidget {
  const new({required this.options});

  final PlayerOptions options;

  @override
  State<_OptionsSheet> createState() => _OptionsSheetState();
}

class _OptionsSheetState extends State<_OptionsSheet> {
  _SheetPage _page = _SheetPage.main;

  PlayerOptions get _options => widget.options;

  @override
  Widget build(BuildContext context) => SafeArea(
    child: ListenableBuilder(
      listenable: Listenable.merge([_options.playback, _options.chat]),
      builder: (context, _) => ListView(
        shrinkWrap: true,
        children: switch ((_page, _options.playback.state)) {
          (
            _SheetPage.quality,
            PlaybackUpdateReady(:final qualities, :final selected),
          ) =>
            _quality(qualities, selected),
          (_SheetPage.sleep, PlaybackUpdateReady(:final sleep)) => _sleep(
            sleep,
          ),
          (_, final state) => _main(state),
        },
      ),
    ),
  );

  List<Widget> _main(PlaybackUpdate state) {
    final chat = _options.chat;
    return [
      if (state case PlaybackUpdateReady(
        :final qualities,
        :final selected,
        :final sleep,
      )) ...[
        ListTile(
          leading: const Icon(Icons.high_quality_outlined),
          title: const Text('Quality'),
          trailing: _value(
            context,
            qualities.where((q) => q.name == selected).map((q) => q.label),
          ),
          onTap: () => _open(_SheetPage.quality),
        ),
        ListTile(
          leading: Icon(
            sleep.left == null ? Icons.bedtime_outlined : Icons.bedtime,
          ),
          title: const Text('Sleep timer'),
          trailing: _value(context, [sleep.left ?? 'Off']),
          onTap: () => _open(_SheetPage.sleep),
        ),
      ],
      if (chat.mode != ChatMode.off)
        SwitchListTile(
          secondary: const Icon(Icons.chat_bubble_outline),
          title: const Text('Chat'),
          value: chat.mode == ChatMode.shown,
          onChanged: (_) => chat.toggle(),
        ),
      ListTile(
        leading: const Icon(Icons.fit_screen_outlined),
        title: const Text('Theatre mode'),
        onTap: () => _choose(_options.onToggleTheatre),
      ),
    ];
  }

  List<Widget> _quality(List<QualityOption> qualities, String selected) => [
    _back('Quality'),
    for (final q in qualities)
      ListTile(
        leading: _check(checked: q.name == selected),
        title: Text(q.label),
        onTap: () => _choose(() => _options.onSelectQuality(q.name)),
      ),
  ];

  List<Widget> _sleep(PlaybackSleep sleep) => [
    _back('Sleep timer'),
    if (sleep.left case final left?)
      ListTile(enabled: false, title: Text(left)),
    for (final preset in sleep.presets)
      ListTile(
        title: Text(preset.label),
        onTap: () => _choose(() => _options.onArmSleep(preset.minutes)),
      ),
    if (sleep.left != null)
      ListTile(
        title: const Text('Turn off timer'),
        onTap: () => _choose(_options.onTurnOffSleep),
      ),
  ];

  /// A sub-list's header, back to the main list.
  Widget _back(String title) => ListTile(
    leading: const Icon(Icons.arrow_back),
    title: Text(title, style: Theme.of(context).textTheme.titleMedium),
    onTap: () => _open(_SheetPage.main),
  );

  /// A main row's current value, then the way into its list.
  Widget _value(BuildContext context, Iterable<String> value) => Row(
    mainAxisSize: MainAxisSize.min,
    children: [
      for (final v in value)
        Text(v, style: Theme.of(context).textTheme.bodyMedium),
      const Icon(Icons.chevron_right),
    ],
  );

  void _open(_SheetPage page) => setState(() => _page = page);

  /// Closes the sheet, then does [action].
  void _choose(VoidCallback action) {
    Navigator.pop(context);
    action();
  }
}
