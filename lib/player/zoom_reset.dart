import 'package:flutter/material.dart';
import 'package:jitter/player/video_zoom.dart';

/// While [zoom] is zoomed in, how far, as a button back to the whole
/// picture; nothing otherwise.
class ZoomReset extends StatelessWidget {
  const new({required this.zoom, super.key});

  final ValueNotifier<VideoZoom> zoom;

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder(
      valueListenable: zoom,
      builder: (context, value, _) {
        if (!value.zoomed) return const SizedBox.shrink();
        return FilledButton.tonalIcon(
          onPressed: () => zoom.value = VideoZoom.none,
          icon: const Icon(Icons.close),
          label: Text(
            '${value.scale.toStringAsFixed(1)}×',
            semanticsLabel:
                'Zoomed ${value.scale.toStringAsFixed(1)} times. '
                'Reset zoom',
          ),
        );
      },
    );
  }
}
