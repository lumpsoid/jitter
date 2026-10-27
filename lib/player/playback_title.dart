import 'package:flutter/material.dart';
import 'package:jitter/src/bindings/bindings.dart';

/// The player page's app bar title: the channel's or the VOD's name, with
/// a line under it once Rust has sent the [details]: for a live stream a
/// LIVE badge, its viewers and uptime; the game being played; and, for a
/// VOD, the day it was streamed. Once the channel is known, its
/// avatar leads and its display name replaces [title]; tapping either
/// calls [onOpenChannel] with its login.
class PlaybackTitle extends StatelessWidget {
  const new({
    required this.title,
    required this.onOpenChannel,
    this.details,
    super.key,
  });

  /// Shown until Rust has sent the channel.
  final String title;

  /// What is playing; `null` until playback is ready.
  final PlaybackDetails? details;

  /// Opens the channel being watched, by login.
  final ValueChanged<String> onOpenChannel;

  @override
  Widget build(BuildContext context) {
    final channel = details?.channel;
    final name = channel?.displayName ?? title;
    final live = details?.live;
    final subtitle = [
      live?.viewers,
      live?.uptime,
      details?.game,
      details?.streamed,
    ].nonNulls;
    final style = Theme.of(context).textTheme.bodySmall;
    final line = Text(
      subtitle.join(' · '),
      overflow: TextOverflow.ellipsis,
      style: style,
    );
    final text = subtitle.isEmpty
        ? Text(name)
        : Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(name, overflow: TextOverflow.ellipsis),
              if (live == null)
                line
              else
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    _LiveBadge(style: style),
                    const SizedBox(width: 6),
                    Flexible(child: line),
                  ],
                ),
            ],
          );
    if (channel == null) return text;
    return Tooltip(
      message: 'Open channel',
      child: InkWell(
        onTap: () => onOpenChannel(channel.login),
        borderRadius: BorderRadius.circular(8),
        child: Padding(
          padding: const EdgeInsets.symmetric(vertical: 4),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              CircleAvatar(
                radius: 16,
                foregroundImage: switch (channel.avatarUrl) {
                  final url? => NetworkImage(url),
                  null => null,
                },
                // A failed avatar falls back to the icon.
                onForegroundImageError: channel.avatarUrl == null
                    ? null
                    : (_, _) {},
                child: const Icon(Icons.person, size: 18),
              ),
              const SizedBox(width: 12),
              Flexible(child: text),
            ],
          ),
        ),
      ),
    );
  }
}

/// Marks the stream as live, ahead of its viewers and uptime.
class _LiveBadge extends StatelessWidget {
  const new({this.style});

  final TextStyle? style;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: colors.error,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 4),
        child: Text(
          'LIVE',
          style: style?.copyWith(
            color: colors.onError,
            fontWeight: FontWeight.bold,
          ),
        ),
      ),
    );
  }
}
