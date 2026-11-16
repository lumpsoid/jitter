import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

/// How far ↑/↓ move the volume (of 0–100), as media_kit's keys do.
const double _volumeStep = 5;

/// The keys of a live stream's desktop controls: media_kit's map without
/// any seek, since a live stream cannot seek. Space and the media keys
/// play and pause, ↑/↓ the volume (through the page's fade, as the wheel),
/// `f` fullscreen and Escape out of it, as media_kit's do. The controls
/// hold the focus while the video has it, so these come before the page's.
Map<ShortcutActivator, VoidCallback> liveKeys({
  required VoidCallback onTogglePlay,
  required VoidCallback onPlay,
  required VoidCallback onPause,
  required ValueChanged<double> onNudgeVolume,
  required VoidCallback onToggleFullscreen,
  required VoidCallback onExitFullscreen,
}) => {
  const SingleActivator(LogicalKeyboardKey.mediaPlay): onPlay,
  const SingleActivator(LogicalKeyboardKey.mediaPause): onPause,
  const SingleActivator(LogicalKeyboardKey.mediaPlayPause): onTogglePlay,
  const SingleActivator(LogicalKeyboardKey.space): onTogglePlay,
  const SingleActivator(LogicalKeyboardKey.arrowUp): () =>
      onNudgeVolume(_volumeStep),
  const SingleActivator(LogicalKeyboardKey.arrowDown): () =>
      onNudgeVolume(-_volumeStep),
  const SingleActivator(LogicalKeyboardKey.keyF): onToggleFullscreen,
  const SingleActivator(LogicalKeyboardKey.escape): onExitFullscreen,
};
