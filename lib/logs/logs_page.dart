import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:jitter/logs/logs_holder.dart';
import 'package:jitter/logs/rust_log.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:jitter/widgets/escape_goes_back.dart';
import 'package:share_plus/share_plus.dart';
import 'package:url_launcher/url_launcher.dart';

/// Whether the platform's share sheet takes a file; elsewhere the page
/// opens the logs' folder instead.
final bool _sharesFiles = Platform.isAndroid || Platform.isIOS;

/// The kept sessions' logs, newest first, one per run of the app. A
/// session opens on [LogSessionPage].
class LogsPage extends StatefulWidget {
  const new({required this.logs, super.key});

  final LogsHolder logs;

  /// Pushes the page, asking Rust for the sessions.
  static Future<void> push(BuildContext context, {required LogsHolder logs}) {
    logs.refresh();
    return Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (context) => EscapeGoesBack(child: LogsPage(logs: logs)),
      ),
    );
  }

  @override
  State<LogsPage> createState() => _LogsPageState();
}

class _LogsPageState extends State<LogsPage> {
  late final StreamSubscription<String> _failures;

  @override
  void initState() {
    super.initState();
    _failures = widget.logs.failures.listen(_showFailure);
  }

  void _showFailure(String message) => _showSnackBar(context, message);

  @override
  void dispose() {
    unawaited(_failures.cancel());
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: widget.logs,
      builder: (context, _) {
        final sessions = widget.logs.sessions;
        return Scaffold(
          appBar: AppBar(
            title: const Text('Logs'),
            actions: [
              if (!_sharesFiles && sessions != null)
                IconButton(
                  tooltip: 'Open folder',
                  icon: const Icon(Icons.folder_open_outlined),
                  onPressed: () => unawaited(_openFolder(sessions.dir)),
                ),
            ],
          ),
          body: sessions == null
              ? const Center(child: CircularProgressIndicator())
              : ListView(
                  children: [
                    const Padding(
                      padding: EdgeInsets.fromLTRB(16, 16, 16, 8),
                      child: Text(
                        'Each run of the app writes one log. To report a '
                        'problem, attach the log of the run it happened in.',
                      ),
                    ),
                    for (final session in sessions.sessions)
                      ListTile(
                        leading: const Icon(Icons.article_outlined),
                        title: Text(session.label),
                        subtitle: session.current
                            ? const Text('This run')
                            : null,
                        onTap: () => unawaited(
                          LogSessionPage.push(
                            context,
                            session: session,
                            logs: widget.logs,
                          ),
                        ),
                      ),
                  ],
                ),
        );
      },
    );
  }
}

/// One session's log: any part of it can be selected and copied, all of
/// it copied, and its file shared (or its folder opened, where the share
/// sheet takes no files).
class LogSessionPage extends StatefulWidget {
  const new({required this.session, required this.logs, super.key});

  final LogSession session;
  final LogsHolder logs;

  /// Pushes the page, asking Rust for the session's log.
  static Future<void> push(
    BuildContext context, {
    required LogSession session,
    required LogsHolder logs,
  }) {
    logs.open(session.name);
    return Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (context) => EscapeGoesBack(
          child: LogSessionPage(session: session, logs: logs),
        ),
      ),
    );
  }

  @override
  State<LogSessionPage> createState() => _LogSessionPageState();
}

class _LogSessionPageState extends State<LogSessionPage> {
  late final StreamSubscription<String> _failures;

  @override
  void initState() {
    super.initState();
    _failures = widget.logs.failures.listen(_showFailure);
  }

  void _showFailure(String message) => _showSnackBar(context, message);

  @override
  void dispose() {
    unawaited(_failures.cancel());
    super.dispose();
  }

  Future<void> _copyAll(String text) async {
    await Clipboard.setData(ClipboardData(text: text));
    if (mounted) _showSnackBar(context, 'Log copied');
  }

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: widget.logs,
      builder: (context, _) {
        // Another session's answer, from a page left before it arrived,
        // is not this one's.
        final opened = switch (widget.logs.opened) {
          final opened? when opened.name == widget.session.name => opened,
          _ => null,
        };
        return Scaffold(
          appBar: AppBar(
            title: Text(widget.session.label),
            actions: [
              if (opened != null) ...[
                IconButton(
                  tooltip: 'Copy all',
                  icon: const Icon(Icons.copy_all_outlined),
                  onPressed: () => unawaited(_copyAll(opened.text)),
                ),
                if (_sharesFiles)
                  IconButton(
                    tooltip: 'Share',
                    icon: const Icon(Icons.share_outlined),
                    onPressed: () => unawaited(_share(opened.path)),
                  )
                else
                  IconButton(
                    tooltip: 'Open folder',
                    icon: const Icon(Icons.folder_open_outlined),
                    onPressed: () =>
                        unawaited(_openFolder(File(opened.path).parent.path)),
                  ),
              ],
            ],
          ),
          body: opened == null
              ? const Center(child: CircularProgressIndicator())
              : _LogText(lines: widget.logs.lines),
        );
      },
    );
  }
}

/// A log's lines in a monospaced font, selectable across lines.
class _LogText extends StatelessWidget {
  const new({required this.lines});

  final List<String> lines;

  @override
  Widget build(BuildContext context) {
    if (lines.isEmpty) {
      return const Center(child: Text('Nothing logged yet'));
    }
    final style = Theme.of(context).textTheme.bodySmall
        ?.copyWith(fontFamily: 'monospace');
    return SelectionArea(
      child: ListView.builder(
        padding: const EdgeInsets.all(8),
        itemCount: lines.length,
        itemBuilder: (context, index) => Text(lines[index], style: style),
      ),
    );
  }
}

void _showSnackBar(BuildContext context, String message) {
  ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(message)));
}

/// Hands the log file at [path] to the platform's share sheet.
Future<void> _share(String path) async {
  await SharePlus.instance.share(ShareParams(files: [XFile(path)]));
}

/// Opens the folder at [dir] in the platform's file manager. A folder it
/// cannot open does nothing.
Future<void> _openFolder(String dir) async {
  try {
    await launchUrl(Uri.directory(dir));
  } on Exception catch (error) {
    logToRust(DartLogLevel.warning, 'logs', 'Could not open $dir: $error');
  }
}
