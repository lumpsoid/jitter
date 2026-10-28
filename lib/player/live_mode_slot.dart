import 'package:flutter/material.dart';
import 'package:jitter/player/live_badge.dart';
import 'package:jitter/player/rewind_messages.dart';
import 'package:jitter/src/bindings/bindings.dart';

const _white = Color(0xFFFFFFFF);
const _muted = Color(0x8AFFFFFF);

/// A live stream's mode in the player's button bar, as Rust sends it in
/// [rewind]: at the live edge, a LIVE badge and a button to rewind the
/// stream into its recording; rewound, a grey LIVE button back to the live
/// edge. Nothing for a VOD.
///
/// While Rust loads either way the button shows it and takes no tap. When
/// Rust says a rewind isn't possible the button is muted but still takes a
/// tap, which shows why in [RewindMessages]: a tooltip alone would need a
/// long press on a phone.
class LiveModeSlot extends StatelessWidget {
  const new({
    required this.rewind,
    required this.onRewind,
    required this.onBackToLive,
    super.key,
  });

  final PlaybackRewind rewind;

  /// Asks Rust to rewind the live stream.
  final VoidCallback onRewind;

  /// Asks Rust to go back to the live edge.
  final VoidCallback onBackToLive;

  @override
  Widget build(BuildContext context) => switch (rewind) {
    PlaybackRewindOffered(:final lastFailure) => _AtLive(
      button: _RewindButton(onPressed: onRewind, tooltip: lastFailure),
    ),
    PlaybackRewindLoading() => const _AtLive(button: _RewindButton()),
    PlaybackRewindUnavailable(:final reason) => _AtLive(
      button: _RewindButton(
        onPressed: () => RewindMessages.show(context, reason),
        tooltip: reason,
        muted: true,
      ),
    ),
    PlaybackRewindRewound() => _BackToLive(onPressed: onBackToLive),
    PlaybackRewindReturning() => const _BackToLive(),
    // A VOD's page.
    _ => const SizedBox.shrink(),
  };
}

TextStyle? _badgeStyle(BuildContext context) =>
    Theme.of(context).textTheme.bodySmall;

/// A spinner the size of the button's icon.
class _Busy extends StatelessWidget {
  const new();

  @override
  Widget build(BuildContext context) => const SizedBox.square(
    dimension: 16,
    child: CircularProgressIndicator(strokeWidth: 2, color: _white),
  );
}

/// The red LIVE badge, then [button].
class _AtLive extends StatelessWidget {
  const new({required this.button});

  final Widget button;

  @override
  Widget build(BuildContext context) => Row(
    mainAxisSize: MainAxisSize.min,
    children: [
      const SizedBox(width: 8),
      LiveBadge(style: _badgeStyle(context)),
      const SizedBox(width: 4),
      button,
    ],
  );
}

class _RewindButton extends StatelessWidget {
  const new({this.onPressed, this.tooltip, this.muted = false});

  /// `null` while Rust loads: it shows a spinner and takes no tap.
  final VoidCallback? onPressed;
  final String? tooltip;
  final bool muted;

  @override
  Widget build(BuildContext context) {
    final button = TextButton.icon(
      onPressed: onPressed,
      style: TextButton.styleFrom(
        foregroundColor: muted ? _muted : _white,
        disabledForegroundColor: _white,
      ),
      icon: onPressed == null ? const _Busy() : const Icon(Icons.history),
      label: const Text('Rewind'),
    );
    return switch (tooltip) {
      final message? => Tooltip(message: message, child: button),
      null => button,
    };
  }
}

/// The grey LIVE badge as a button back to the live edge.
class _BackToLive extends StatelessWidget {
  const new({this.onPressed});

  /// `null` while Rust loads: it shows a spinner and takes no tap.
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) => Tooltip(
    message: 'Back to live',
    child: TextButton(
      onPressed: onPressed,
      style: TextButton.styleFrom(
        foregroundColor: _white,
        disabledForegroundColor: _white,
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          LiveBadge(style: _badgeStyle(context), behind: true),
          const SizedBox(width: 6),
          if (onPressed == null)
            const _Busy()
          else
            const Icon(Icons.last_page, size: 20),
        ],
      ),
    ),
  );
}
