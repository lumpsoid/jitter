import 'dart:io';

import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher_string.dart';

/// Asks a running Steam to show its on-screen keyboard.
const _steamKeyboardUrl = 'steam://open/keyboard';

/// Whether this runs where Steam's on-screen keyboard can be asked for:
/// launched by Steam on a Deck or on SteamOS (Steam sets `SteamDeck`,
/// `SteamOS` or, in Game Mode, `SteamGamepadUI` to `1` for what it
/// launches), or on SteamOS at all, by its [osRelease]
/// (`/etc/os-release`), where Steam always runs. Elsewhere the URL would
/// start Steam rather than show a keyboard.
bool steamKeyboardAvailable({
  required Map<String, String> environment,
  required String? osRelease,
}) {
  const launchedBySteam = ['SteamDeck', 'SteamOS', 'SteamGamepadUI'];
  if (launchedBySteam.any((name) => environment[name] == '1')) return true;
  if (osRelease == null) return false;
  for (final line in osRelease.split('\n')) {
    final at = line.indexOf('=');
    if (at < 0 || line.substring(0, at).trim() != 'ID') continue;
    final id = line.substring(at + 1).trim().replaceAll(RegExp(r'^"|"$'), '');
    return id == 'steamos';
  }
  return false;
}

/// [steamKeyboardAvailable] for this process, read once.
final bool _available =
    Platform.isLinux &&
    steamKeyboardAvailable(
      environment: Platform.environment,
      osRelease: _readOsRelease(),
    );

String? _readOsRelease() {
  try {
    return File('/etc/os-release').readAsStringSync();
  } on FileSystemException {
    return null;
  }
}

/// A keyboard button for a text field's suffix that focuses the [field] and
/// shows Steam's on-screen keyboard, which types into it; where Steam's
/// keyboard is not there to show ([steamKeyboardAvailable]), nothing.
class SteamKeyboardButton extends StatelessWidget {
  const new({required this.field, super.key});

  final FocusNode field;

  /// Whether the button shows here; a field lays out its suffix by it.
  static bool get shown => _available;

  @override
  Widget build(BuildContext context) {
    if (!_available) return const SizedBox.shrink();
    return IconButton(
      tooltip: 'On-screen keyboard',
      icon: const Icon(Icons.keyboard),
      onPressed: () async {
        field.requestFocus();
        await launchUrlString(_steamKeyboardUrl);
      },
    );
  }
}
