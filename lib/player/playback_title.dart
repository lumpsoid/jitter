import 'package:flutter/material.dart';
import 'package:jitter/src/bindings/bindings.dart';

/// The player page's app bar title: the channel's or the VOD's name, with
/// a line under it once Rust has sent the [details]: the game being played
/// and, for a VOD, the day it was streamed. Once the channel is known, its
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
    final subtitle = [details?.game, details?.streamed].nonNulls;
    final text = subtitle.isEmpty
        ? Text(name)
        : Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(name, overflow: TextOverflow.ellipsis),
              Text(
                subtitle.join(' · '),
                overflow: TextOverflow.ellipsis,
                style: Theme.of(context).textTheme.bodySmall,
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
