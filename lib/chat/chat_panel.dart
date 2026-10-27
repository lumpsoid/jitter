import 'dart:async';

import 'package:flutter/material.dart';
import 'package:jitter/chat/chat_entry.dart';
import 'package:jitter/chat/chat_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:url_launcher/url_launcher_string.dart';

/// Scrolled further up than this, the list stops following new messages.
const double _pauseOffset = 24;

/// Emotes are drawn this tall, about two lines of text.
const double _emoteHeight = 28;

/// The chat, live or a VOD's replay: its status lines, then the messages,
/// newest at the bottom, then the connection bar while not connected (or
/// while a replay loads), where new messages would arrive. Follows new
/// messages until the user scrolls up; the list then holds still until they
/// scroll back down or tap "Latest messages".
class ChatPanel extends StatefulWidget {
  const new({required this.chat, super.key});

  final ChatHolder chat;

  @override
  State<ChatPanel> createState() => _ChatPanelState();
}

class _ChatPanelState extends State<ChatPanel> {
  final _scroll = ScrollController();

  /// The messages as they were when the user scrolled up; null while the
  /// list follows new ones.
  List<ChatEntry>? _paused;

  @override
  void initState() {
    super.initState();
    _scroll.addListener(_onScroll);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    widget.chat.setDark(dark: Theme.of(context).brightness == Brightness.dark);
  }

  void _onScroll() {
    final away = _scroll.offset > _pauseOffset;
    if (away == (_paused != null)) return;
    setState(() => _paused = away ? List.of(widget.chat.messages) : null);
  }

  void _toLatest() => _scroll.jumpTo(0);

  @override
  void dispose() {
    _scroll.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: widget.chat,
      builder: (context, _) {
        final status = widget.chat.status;
        final messages = _paused ?? widget.chat.messages;
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            if (status.modes.isNotEmpty)
              _StatusLine(text: status.modes.join(' · '), icon: Icons.tune),
            if (status.notice case final notice?)
              _StatusLine(text: notice, icon: Icons.info_outline),
            if (status.emoteHint case final hint?)
              _StatusLine(text: hint, icon: Icons.image_not_supported_outlined),
            Expanded(
              child: Stack(
                children: [
                  ListView.builder(
                    controller: _scroll,
                    reverse: true,
                    padding: const EdgeInsets.symmetric(vertical: 4),
                    itemCount: messages.length,
                    itemBuilder: (context, i) =>
                        _Message(entry: messages[messages.length - 1 - i]),
                  ),
                  if (_paused != null)
                    Positioned(
                      left: 0,
                      right: 0,
                      bottom: 8,
                      child: Center(
                        child: ActionChip(
                          avatar: const Icon(Icons.arrow_downward, size: 16),
                          label: const Text('Latest messages'),
                          onPressed: _toLatest,
                        ),
                      ),
                    ),
                ],
              ),
            ),
            if (status.banner case final banner?)
              _StatusLine(text: banner, icon: Icons.sync),
          ],
        );
      },
    );
  }
}

/// One slim line around the messages: the connection below them; room
/// modes, a notice, or the missing-emotes hint above.
class _StatusLine extends StatelessWidget {
  const new({required this.text, required this.icon});

  final String text;
  final IconData icon;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final color = theme.colorScheme.onSurfaceVariant;
    return ColoredBox(
      color: theme.colorScheme.surfaceContainerHigh,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        child: Row(
          children: [
            Icon(icon, size: 14, color: color),
            const SizedBox(width: 6),
            Expanded(
              child: Text(
                text,
                style: theme.textTheme.bodySmall?.copyWith(color: color),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// `name: body`, or `name body` in the name's colour for a `/me`. A
/// deleted message keeps its name, then reads "(deleted)" and strikes its
/// body through in a muted, still readable colour. The marker comes before
/// the body so a screen reader announces it first, ahead of any link, which
/// is its own semantics node.
class _Message extends StatelessWidget {
  const new({required this.entry});

  final ChatEntry entry;

  @override
  Widget build(BuildContext context) {
    final line = entry.line;
    final deleted = entry.deleted;
    final colors = Theme.of(context).colorScheme;
    final nameColor = Color(line.color);
    final style = DefaultTextStyle.of(context).style;
    final body = switch ((deleted, line.action)) {
      (true, _) => style.copyWith(
        color: colors.onSurfaceVariant,
        decoration: TextDecoration.lineThrough,
      ),
      (false, true) => style.copyWith(color: nameColor),
      (false, false) => style,
    };
    final linkStyle = body.copyWith(
      color: colors.primary,
      decoration: TextDecoration.combine([
        TextDecoration.underline,
        if (deleted) TextDecoration.lineThrough,
      ]),
    );
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
      child: Text.rich(
        TextSpan(
          children: [
            TextSpan(
              text: line.name,
              style: TextStyle(color: nameColor, fontWeight: FontWeight.bold),
            ),
            TextSpan(text: line.action ? ' ' : ': '),
            if (deleted)
              TextSpan(
                text: '(deleted) ',
                style: style.copyWith(
                  color: colors.onSurfaceVariant,
                  fontStyle: FontStyle.italic,
                ),
              ),
            for (final fragment in line.fragments)
              _fragment(fragment, body, linkStyle),
          ],
        ),
        style: body,
      ),
    );
  }

  InlineSpan _fragment(ChatFragment fragment, TextStyle body, TextStyle link) {
    return switch (fragment) {
      ChatFragmentText(:final text) => TextSpan(text: text, style: body),
      ChatFragmentEmote(:final emote) => WidgetSpan(
        alignment: PlaceholderAlignment.middle,
        child: _Emote(emote: emote),
      ),
      ChatFragmentOverlay(:final base, :final overlays) => WidgetSpan(
        alignment: PlaceholderAlignment.middle,
        child: Stack(
          alignment: Alignment.center,
          children: [
            _Emote(emote: base),
            for (final overlay in overlays) _Emote(emote: overlay),
          ],
        ),
      ),
      ChatFragmentLink(:final text, :final url) => WidgetSpan(
        alignment: PlaceholderAlignment.baseline,
        baseline: TextBaseline.alphabetic,
        child: _Link(text: text, url: url, style: link),
      ),
      _ => const TextSpan(),
    };
  }
}

/// A link in a message: opens on a click, a tap, or Enter/Space once
/// focused with Tab, and is announced as a link. Focus tints it.
class _Link extends StatelessWidget {
  const new({required this.text, required this.url, required this.style});

  final String text;
  final String url;
  final TextStyle style;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      // Its own node, or the whole message would read as the link.
      container: true,
      link: true,
      linkUrl: Uri.tryParse(url),
      // Its own ink layer, so the tint draws inside the message's text
      // rather than on a Material further up.
      child: Material(
        type: MaterialType.transparency,
        child: InkWell(
          onTap: () => unawaited(_open(url)),
          focusColor: Theme.of(context).colorScheme.primary
              .withValues(alpha: 0.24),
          child: Text(text, style: style),
        ),
      ),
    );
  }
}

class _Emote extends StatelessWidget {
  const new({required this.emote});

  final EmoteImage emote;

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: emote.name,
      child: Image.network(
        emote.url,
        height: _emoteHeight,
        errorBuilder: (context, error, stack) => Text(emote.name),
      ),
    );
  }
}

/// Opens a chat link in the platform's browser, a platform effect. A link
/// the platform cannot open does nothing.
Future<void> _open(String url) async {
  try {
    await launchUrlString(url, mode: LaunchMode.externalApplication);
  } on Exception catch (error) {
    debugPrint('Could not open $url: $error');
  }
}
