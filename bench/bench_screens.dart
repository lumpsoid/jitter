import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

/// The setup form: what to play, on which engine, and for how long. The
/// fields belong to the bench app, which reads them when Run is pressed.
class SetupScreen extends StatelessWidget {
  const new({
    required this.buildId,
    required this.targets,
    required this.hwdec,
    required this.warmup,
    required this.measured,
    required this.engine,
    required this.locked,
    required this.running,
    required this.canRun,
    required this.status,
    required this.hasLog,
    required this.onEngine,
    required this.onTargetsChanged,
    required this.onRun,
    required this.onShowLog,
    super.key,
  });

  final String buildId;
  final TextEditingController targets;
  final TextEditingController hwdec;
  final TextEditingController warmup;
  final TextEditingController measured;
  final String engine;

  /// Whether a run has opened the engine, which then stays for this launch.
  final bool locked;
  final bool running;
  final bool canRun;
  final String status;
  final bool hasLog;
  final ValueChanged<String> onEngine;
  final ValueChanged<String> onTargetsChanged;
  final VoidCallback onRun;
  final VoidCallback onShowLog;

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    final frozen = locked || running;
    return Scaffold(
      body: ListView(
        padding: const EdgeInsets.all(24),
        children: [
          Text('Jitter player bench', style: text.headlineSmall),
          Text('build $buildId', style: text.bodySmall),
          const SizedBox(height: 24),
          TextField(
            controller: targets,
            decoration: const InputDecoration(
              labelText: 'Channels, VOD ids or links',
              helperText:
                  'One entry plays 3 times; several play once each, '
                  'in order.',
            ),
            onChanged: onTargetsChanged,
          ),
          const SizedBox(height: 24),
          SegmentedButton<String>(
            segments: [
              const ButtonSegment(value: 'mpv', label: Text('mpv (media_kit)')),
              const ButtonSegment(value: 'fvp', label: Text('fvp (libmdk)')),
              if (Platform.isAndroid)
                const ButtonSegment(value: 'exo', label: Text('exo')),
            ],
            selected: {engine},
            onSelectionChanged: frozen
                ? null
                : (picked) => onEngine(picked.single),
          ),
          if (locked) ...[
            const SizedBox(height: 8),
            const Text('To test another engine, close and restart the app.'),
          ],
          if (engine == 'mpv') ...[
            const SizedBox(height: 16),
            TextField(
              controller: hwdec,
              enabled: !frozen,
              decoration: const InputDecoration(
                labelText: 'mpv hwdec (blank = default)',
              ),
            ),
          ],
          const SizedBox(height: 16),
          TextField(
            controller: warmup,
            keyboardType: TextInputType.number,
            inputFormatters: [FilteringTextInputFormatter.digitsOnly],
            decoration: const InputDecoration(labelText: 'Warm-up (s)'),
          ),
          const SizedBox(height: 16),
          TextField(
            controller: measured,
            keyboardType: TextInputType.number,
            inputFormatters: [FilteringTextInputFormatter.digitsOnly],
            decoration: const InputDecoration(labelText: 'Measured (s)'),
          ),
          const SizedBox(height: 24),
          const Text(
            'Maximize this window first, and keep it in front and visible '
            'until the results show.',
          ),
          const SizedBox(height: 24),
          FilledButton(
            onPressed: canRun ? onRun : null,
            child: const Text('Run test'),
          ),
          if (status.isNotEmpty) Text(status),
          if (hasLog) ...[
            const SizedBox(height: 16),
            OutlinedButton(
              onPressed: running ? null : onShowLog,
              child: const Text('Show log'),
            ),
          ],
        ],
      ),
    );
  }
}

/// The run's picture, full-window on black, with its status in the corner.
class PlayingScreen extends StatelessWidget {
  const new({required this.picture, required this.status, super.key});

  /// The engine's picture, or nothing when it has none yet.
  final Widget? picture;
  final String status;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Colors.black,
      body: Stack(
        children: [
          Center(child: picture ?? const SizedBox.shrink()),
          Align(
            alignment: Alignment.topLeft,
            child: ColoredBox(
              color: Colors.black54,
              child: Padding(
                padding: const EdgeInsets.all(6),
                child: Text(
                  status,
                  style: const TextStyle(color: Colors.white, fontSize: 12),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// The whole log, selectable, to copy back into the chat.
class ResultScreen extends StatelessWidget {
  const new({
    required this.log,
    required this.onCopy,
    required this.onBack,
    super.key,
  });

  final String log;
  final VoidCallback onCopy;
  final VoidCallback onBack;

  @override
  Widget build(BuildContext context) {
    final family = Platform.isWindows ? 'Consolas' : 'monospace';
    return Scaffold(
      appBar: AppBar(title: const Text('Result')),
      body: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Expanded(
              child: SingleChildScrollView(
                child: SelectableText(
                  log,
                  style: TextStyle(fontFamily: family, fontSize: 12),
                ),
              ),
            ),
            const SizedBox(height: 12),
            Row(
              children: [
                FilledButton(onPressed: onCopy, child: const Text('Copy log')),
                const SizedBox(width: 12),
                OutlinedButton(onPressed: onBack, child: const Text('Back')),
              ],
            ),
            const SizedBox(height: 12),
            const Text('Copy the log and paste it back into the chat.'),
          ],
        ),
      ),
    );
  }
}
