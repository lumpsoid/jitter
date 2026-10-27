import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

/// How far ←/→ (and J/I) move a VOD: media_kit's double tap on a side of
/// a phone's video goes the same.
const vodSeekStep = Duration(seconds: 10);

/// How far Shift with ←/→ moves it.
const _fineSeekStep = Duration(seconds: 1);

/// How far ↑/↓ move the volume (of 0–100), as media_kit's keys do.
const double _volumeStep = 5;

/// The keys of a VOD's desktop controls, in place of media_kit's own: its
/// map with ←/→ 10 s instead of 2, Shift with them 1 s, and `,`/`.` a frame
/// back and forward while paused. Space and the media keys play and pause,
/// J/I go 10 s, ↑/↓ the volume (through the page's fade, as the wheel),
/// `f` fullscreen and Escape out of it, as media_kit's do. The controls
/// hold the focus while the video has it, so these come before the page's.
Map<ShortcutActivator, VoidCallback> vodKeys({
  required VoidCallback onTogglePlay,
  required VoidCallback onPlay,
  required VoidCallback onPause,
  required ValueChanged<Duration> onSeekBy,
  required ValueChanged<bool> onStepFrame,
  required ValueChanged<double> onNudgeVolume,
  required VoidCallback onToggleFullscreen,
  required VoidCallback onExitFullscreen,
}) => {
  const SingleActivator(LogicalKeyboardKey.mediaPlay): onPlay,
  const SingleActivator(LogicalKeyboardKey.mediaPause): onPause,
  const SingleActivator(LogicalKeyboardKey.mediaPlayPause): onTogglePlay,
  const SingleActivator(LogicalKeyboardKey.space): onTogglePlay,
  const SingleActivator(LogicalKeyboardKey.arrowLeft): () =>
      onSeekBy(-vodSeekStep),
  const SingleActivator(LogicalKeyboardKey.arrowRight): () =>
      onSeekBy(vodSeekStep),
  const SingleActivator(LogicalKeyboardKey.arrowLeft, shift: true): () =>
      onSeekBy(-_fineSeekStep),
  const SingleActivator(LogicalKeyboardKey.arrowRight, shift: true): () =>
      onSeekBy(_fineSeekStep),
  const SingleActivator(LogicalKeyboardKey.keyJ): () => onSeekBy(-vodSeekStep),
  const SingleActivator(LogicalKeyboardKey.keyI): () => onSeekBy(vodSeekStep),
  // By character, so they are found wherever the layout puts them.
  const CharacterActivator(','): () => onStepFrame(false),
  const CharacterActivator('.'): () => onStepFrame(true),
  const SingleActivator(LogicalKeyboardKey.arrowUp): () =>
      onNudgeVolume(_volumeStep),
  const SingleActivator(LogicalKeyboardKey.arrowDown): () =>
      onNudgeVolume(-_volumeStep),
  const SingleActivator(LogicalKeyboardKey.keyF): onToggleFullscreen,
  const SingleActivator(LogicalKeyboardKey.escape): onExitFullscreen,
};
