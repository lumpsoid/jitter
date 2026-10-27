import 'package:flutter/material.dart';
import 'package:jitter/src/bindings/bindings.dart';

/// The player page's app bar title: the channel's or the VOD's name, with
/// the game being played under it once Rust has sent the [details].
class PlaybackTitle extends StatelessWidget {
  const new({required this.title, this.details, super.key});

  final String title;

  /// What is playing; `null` until playback is ready.
  final PlaybackDetails? details;

  @override
  Widget build(BuildContext context) {
    final game = details?.game;
    if (game == null) return Text(title);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(title, overflow: TextOverflow.ellipsis),
        Text(
          game,
          overflow: TextOverflow.ellipsis,
          style: Theme.of(context).textTheme.bodySmall,
        ),
      ],
    );
  }
}
