import 'package:flutter/material.dart';

/// A channel's About tab: its follower count, when Twitch sent one, above
/// its description whole, or a note that it has none. Pulling down asks
/// [onRefresh] to look the channel up again.
class ChannelAbout extends StatelessWidget {
  const new({
    required this.followersLabel,
    required this.description,
    required this.onRefresh,
    super.key,
  });

  final String? followersLabel;
  final String? description;
  final VoidCallback onRefresh;

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    final followersLabel = this.followersLabel;
    return RefreshIndicator(
      onRefresh: () async => onRefresh(),
      child: ListView(
        physics: const AlwaysScrollableScrollPhysics(),
        padding: const EdgeInsets.all(16),
        children: [
          if (followersLabel != null) ...[
            Text(followersLabel, style: text.titleSmall),
            const SizedBox(height: 12),
          ],
          switch (description) {
            final description? => SelectableText(
              description,
              style: text.bodyMedium,
            ),
            null => Text(
              'No description.',
              style: text.bodyMedium?.copyWith(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ),
          },
        ],
      ),
    );
  }
}
