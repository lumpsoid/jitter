import 'dart:async';

import 'package:flutter/material.dart';
import 'package:jitter/history/history_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';

/// What the history page lists: every visit by day, or each live channel
/// and VOD once. The page's own choice, not Rust's.
enum HistoryView { timeline, unique }

/// What you watched: the timeline of visits by day, newest first, or each
/// live channel and VOD once by its latest visit. Tapping an entry plays
/// it again (a new visit, today); its avatar opens the channel. Each entry
/// can be removed, and the whole history cleared. Shows every removal
/// failure as a snackbar.
class HistoryPage extends StatefulWidget {
  const new({
    required this.history,
    required this.onOpenLive,
    required this.onOpenVod,
    required this.onOpenChannel,
    super.key,
  });

  final HistoryHolder history;

  /// Plays this channel live.
  final void Function(String login) onOpenLive;

  /// Plays this VOD.
  final void Function(String id) onOpenVod;

  /// Opens the channel page for this login.
  final void Function(String login) onOpenChannel;

  @override
  State<HistoryPage> createState() => _HistoryPageState();
}

class _HistoryPageState extends State<HistoryPage> {
  late final StreamSubscription<String> _failures;
  HistoryView _view = HistoryView.timeline;

  @override
  void initState() {
    super.initState();
    _failures = widget.history.failures.listen(_showFailure);
  }

  void _showFailure(String message) {
    ScaffoldMessenger.of(context)
        .showSnackBar(SnackBar(content: Text(message)));
  }

  @override
  void dispose() {
    unawaited(_failures.cancel());
    super.dispose();
  }

  void _play(HistoryEntry entry) {
    final target = entry.target;
    if (target is PlaybackTargetLive) widget.onOpenLive(target.login);
    if (target is PlaybackTargetVod) widget.onOpenVod(target.id);
  }

  Future<void> _confirmClear() async {
    final clear = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Clear the history?'),
        content: const Text('Every visit on every day is removed.'),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text('Cancel'),
          ),
          TextButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: const Text('Clear'),
          ),
        ],
      ),
    );
    if (clear ?? false) widget.history.clear();
  }

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: widget.history,
      builder: (context, _) {
        final history = widget.history.history;
        if (history == null) {
          return const Center(child: CircularProgressIndicator());
        }
        return CustomScrollView(
          slivers: [
            SliverToBoxAdapter(
              child: _Bar(
                view: _view,
                onView: (view) => setState(() => _view = view),
                onClear: history.days.isEmpty ? null : _confirmClear,
              ),
            ),
            if (history.days.isEmpty)
              const SliverToBoxAdapter(child: _NothingWatched())
            else
              ...switch (_view) {
                HistoryView.timeline => [
                  for (final day in history.days) ...[
                    _Header(label: day.label),
                    _entries(day.entries, unique: false),
                  ],
                ],
                HistoryView.unique => [_entries(history.unique, unique: true)],
              },
            const SliverToBoxAdapter(child: SizedBox(height: 16)),
          ],
        );
      },
    );
  }

  Widget _entries(List<HistoryEntry> entries, {required bool unique}) =>
      SliverList.builder(
        itemCount: entries.length,
        itemBuilder: (context, index) {
          final entry = entries[index];
          return _EntryTile(
            entry: entry,
            removeLabel: unique ? 'Remove from history' : 'Remove this visit',
            onTap: () => _play(entry),
            onOpenChannel: widget.onOpenChannel,
            onRemove: () => unique
                ? widget.history.removeFromHistory(entry)
                : widget.history.removeVisit(entry),
          );
        },
      );
}

/// The view switch, and clearing the history ([onClear] null when there
/// is nothing to clear).
class _Bar extends StatelessWidget {
  const new({required this.view, required this.onView, required this.onClear});

  final HistoryView view;
  final ValueChanged<HistoryView> onView;
  final VoidCallback? onClear;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 12, 8, 0),
      child: Row(
        children: [
          SegmentedButton<HistoryView>(
            segments: const [
              ButtonSegment(
                value: HistoryView.timeline,
                label: Text('Timeline'),
                icon: Icon(Icons.timeline),
              ),
              ButtonSegment(
                value: HistoryView.unique,
                label: Text('Unique'),
                icon: Icon(Icons.filter_1_outlined),
              ),
            ],
            selected: {view},
            onSelectionChanged: (chosen) => onView(chosen.single),
          ),
          const Spacer(),
          IconButton(
            tooltip: 'Clear the history',
            icon: const Icon(Icons.delete_sweep_outlined),
            onPressed: onClear,
          ),
        ],
      ),
    );
  }
}

class _NothingWatched extends StatelessWidget {
  const new();

  @override
  Widget build(BuildContext context) {
    return const Padding(
      padding: EdgeInsets.all(32),
      child: Text(
        'Nothing watched yet. What you watch shows up here, day by day.',
        textAlign: TextAlign.center,
      ),
    );
  }
}

class _Header extends StatelessWidget {
  const new({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    return SliverToBoxAdapter(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 16, 16, 4),
        child: Text(label, style: Theme.of(context).textTheme.titleMedium),
      ),
    );
  }
}

/// One visit: the channel's avatar, the title, what it was (live or the
/// VOD's day, the channel, the game), when it started and how long it was
/// watched, and a menu to remove it.
class _EntryTile extends StatelessWidget {
  const new({
    required this.entry,
    required this.removeLabel,
    required this.onTap,
    required this.onOpenChannel,
    required this.onRemove,
  });

  final HistoryEntry entry;
  final String removeLabel;
  final VoidCallback onTap;
  final void Function(String login) onOpenChannel;
  final VoidCallback onRemove;

  @override
  Widget build(BuildContext context) {
    final channel = entry.channel;
    final streamed = entry.streamed;
    final what = [
      if (entry.live)
        'Live'
      else if (streamed != null)
        'VOD · $streamed'
      else
        'VOD',
      ?channel?.displayName,
      ?entry.game,
    ].join(' · ');
    return ListTile(
      onTap: onTap,
      leading: _Avatar(
        channel: channel,
        onTap: channel == null ? null : () => onOpenChannel(channel.login),
      ),
      title: Text(entry.title, maxLines: 2, overflow: TextOverflow.ellipsis),
      subtitle: Text(
        '$what\n${entry.when} · watched ${entry.watched}',
        maxLines: 2,
        overflow: TextOverflow.ellipsis,
      ),
      isThreeLine: true,
      trailing: PopupMenuButton<VoidCallback>(
        tooltip: 'More',
        onSelected: (action) => action(),
        itemBuilder: (context) => [
          PopupMenuItem(value: onRemove, child: Text(removeLabel)),
        ],
      ),
    );
  }
}

/// The channel's avatar over its initial (shown with no avatar, or while
/// it fails to load); a placeholder icon when the channel is not known.
class _Avatar extends StatelessWidget {
  const new({required this.channel, required this.onTap});

  final PlaybackChannel? channel;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final channel = this.channel;
    final url = channel?.avatarUrl;
    return InkWell(
      customBorder: const CircleBorder(),
      onTap: onTap,
      child: CircleAvatar(
        backgroundColor: theme.colorScheme.surfaceContainerHighest,
        foregroundColor: theme.colorScheme.onSurfaceVariant,
        foregroundImage: url == null ? null : NetworkImage(url),
        // A failed avatar falls back to the initial.
        onForegroundImageError: url == null ? null : (_, _) {},
        child: channel == null
            ? const Icon(Icons.history)
            : Text(
                channel.displayName.characters.take(1).toUpperCase().toString(),
              ),
      ),
    );
  }
}
