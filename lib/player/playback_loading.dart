import 'package:flutter/material.dart';

/// The spinner on the black video box while a stream loads. A live region
/// labelled "Loading stream", so a screen reader announces it without the
/// user moving focus.
class PlaybackLoading extends StatelessWidget {
  const new({super.key});

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Semantics(
        container: true,
        liveRegion: true,
        label: 'Loading stream',
        child: const ExcludeSemantics(child: CircularProgressIndicator()),
      ),
    );
  }
}
