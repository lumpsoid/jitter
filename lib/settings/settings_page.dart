import 'dart:async';

import 'package:flutter/material.dart';
import 'package:jitter/logs/logs_holder.dart';
import 'package:jitter/logs/logs_page.dart';
import 'package:jitter/settings/settings_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';

/// The user's settings: the theme, how chat loads and draws emotes, and
/// the quality a player opens at. Each control sends its change to Rust
/// and shows the setting as Rust answers; a change Rust could not store
/// shows as a snackbar. The app's logs open from here, and its version
/// shows at the foot.
class SettingsPage extends StatefulWidget {
  const new({required this.settings, required this.logs, super.key});

  final SettingsHolder settings;
  final LogsHolder logs;

  @override
  State<SettingsPage> createState() => _SettingsPageState();
}

class _SettingsPageState extends State<SettingsPage> {
  late final StreamSubscription<String> _failures;

  @override
  void initState() {
    super.initState();
    _failures = widget.settings.failures.listen(_showFailure);
  }

  void _showFailure(String message) {
    ScaffoldMessenger.of(context)
        .showSnackBar(SnackBar(content: Text(message)));
  }

  @override
  void dispose() {
    unawaited(_failures.cancel());
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: widget.settings,
      builder: (context, _) {
        final settings = widget.settings.settings;
        if (settings == null) {
          return const Center(child: CircularProgressIndicator());
        }
        final holder = widget.settings;
        return ListView(
          children: [
            const _Header(label: 'Appearance'),
            ListTile(
              title: const Text('Theme'),
              subtitle: Padding(
                padding: const EdgeInsets.only(top: 8),
                child: SegmentedButton<AppTheme>(
                  segments: const [
                    ButtonSegment(
                      value: AppTheme.system,
                      label: Text('System'),
                      icon: Icon(Icons.brightness_auto_outlined),
                    ),
                    ButtonSegment(
                      value: AppTheme.light,
                      label: Text('Light'),
                      icon: Icon(Icons.light_mode_outlined),
                    ),
                    ButtonSegment(
                      value: AppTheme.dark,
                      label: Text('Dark'),
                      icon: Icon(Icons.dark_mode_outlined),
                    ),
                  ],
                  selected: {settings.theme},
                  onSelectionChanged: (chosen) =>
                      holder.chooseTheme(chosen.single),
                ),
              ),
            ),
            const _Header(label: 'Chat'),
            SwitchListTile(
              title: const Text('Load chat automatically'),
              subtitle: const Text(
                'When off, the chat starts hidden; show it from the player.',
              ),
              value: settings.autoLoadChat,
              onChanged: (on) => holder.setAutoLoadChat(on: on),
            ),
            SwitchListTile(
              title: const Text('Animated emotes'),
              subtitle: const Text('When off, emotes show as still images.'),
              value: settings.animatedEmotes,
              onChanged: (on) => holder.setAnimatedEmotes(on: on),
            ),
            SwitchListTile(
              title: const Text('Third-party emotes'),
              subtitle: const Text(
                '7TV, BTTV and FFZ emotes, from the next chat you open.',
              ),
              value: settings.thirdPartyEmotes,
              onChanged: (on) => holder.setThirdPartyEmotes(on: on),
            ),
            const _Header(label: 'Starting quality'),
            RadioGroup<StartQuality>(
              groupValue: settings.startQuality,
              onChanged: (quality) {
                if (quality != null) holder.chooseStartQuality(quality);
              },
              child: Column(
                children: [
                  for (final choice in settings.startQualities)
                    RadioListTile<StartQuality>(
                      value: choice.quality,
                      title: Text(choice.label),
                    ),
                ],
              ),
            ),
            const Padding(
              padding: EdgeInsets.fromLTRB(16, 0, 16, 16),
              child: Text(
                'A stream without it opens at the closest lower quality.',
              ),
            ),
            const _Header(label: 'Background playback'),
            SwitchListTile(
              title: const Text('Play audio in the background'),
              subtitle: const Text(
                'Leaving the app keeps the stream going as audio only. '
                'When off, it stops until you come back.',
              ),
              value: settings.backgroundAudio,
              onChanged: (on) => holder.setBackgroundAudio(on: on),
            ),
            const _Header(label: 'Troubleshooting'),
            ListTile(
              leading: const Icon(Icons.article_outlined),
              title: const Text('Logs'),
              subtitle: const Text(
                'What the app recorded on each run, to attach to a bug '
                'report.',
              ),
              onTap: () => unawaited(LogsPage.push(context, logs: widget.logs)),
            ),
            const _Header(label: 'About'),
            ListTile(
              leading: const Icon(Icons.info_outline),
              title: const Text('Version'),
              subtitle: SelectableText(settings.version),
            ),
          ],
        );
      },
    );
  }
}

/// A section's title, announced as a heading.
class _Header extends StatelessWidget {
  const new({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 24, 16, 8),
      child: Semantics(
        header: true,
        child: Text(
          label,
          style: Theme.of(context).textTheme.titleSmall
              ?.copyWith(color: Theme.of(context).colorScheme.primary),
        ),
      ),
    );
  }
}
