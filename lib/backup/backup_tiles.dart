import 'dart:async';
import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:jitter/backup/backup_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:path_provider/path_provider.dart';
import 'package:share_plus/share_plus.dart';

/// Saves a backup's [json] as [fileName] where the user chooses. Returns
/// where it went, or null when the user cancelled or the platform's share
/// sheet took it.
typedef SaveBackup = Future<String?> Function(String fileName, String json);

/// Lets the user pick a backup file: its text, or null when cancelled.
typedef PickBackup = Future<String?> Function();

/// The settings page's export and import tiles. Export asks Rust for a
/// backup and saves it; import picks a file and hands its text to Rust.
/// Rust's answers, and a file that could not be saved or read, show as
/// snackbars.
class BackupTiles extends StatefulWidget {
  const new({
    required this.backup,
    this.save = saveBackup,
    this.pick = pickBackup,
    super.key,
  });

  final BackupHolder backup;

  /// The platform's file effects; tests replace them.
  final SaveBackup save;
  final PickBackup pick;

  @override
  State<BackupTiles> createState() => _BackupTilesState();
}

class _BackupTilesState extends State<BackupTiles> {
  late final StreamSubscription<BackupUpdate> _answers;

  @override
  void initState() {
    super.initState();
    _answers = widget.backup.answers.listen(
      (answer) => unawaited(_answered(answer)),
    );
  }

  @override
  void dispose() {
    unawaited(_answers.cancel());
    super.dispose();
  }

  Future<void> _answered(BackupUpdate answer) async {
    switch (answer) {
      case BackupUpdateExported(:final fileName, :final json):
        try {
          final saved = await widget.save(fileName, json);
          if (saved != null) _show('Backup saved to $saved');
        } on Exception catch (error) {
          _show("Couldn't save the backup: $error");
        }
      case BackupUpdateImported(:final summary):
        _show(summary);
      case BackupUpdateFailed(:final message):
        _show(message);
    }
  }

  Future<void> _import() async {
    try {
      final json = await widget.pick();
      if (json != null) widget.backup.import(json);
    } on Exception catch (error) {
      _show("Couldn't read the backup: $error");
    }
  }

  void _show(String message) {
    if (!mounted) return;
    ScaffoldMessenger.of(context)
        .showSnackBar(SnackBar(content: Text(message)));
  }

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: widget.backup,
      builder: (context, _) {
        final idle = !widget.backup.busy;
        return Column(
          children: [
            ListTile(
              leading: const Icon(Icons.save_alt_outlined),
              title: const Text('Export data'),
              subtitle: const Text(
                'Save your favorites, history, VOD positions and settings '
                'to a file.',
              ),
              enabled: idle,
              onTap: widget.backup.export,
            ),
            ListTile(
              leading: const Icon(Icons.file_open_outlined),
              title: const Text('Import data'),
              subtitle: const Text(
                "Add a backup's favorites, history and VOD positions to "
                "this app's, and take its settings. Nothing is removed.",
              ),
              enabled: idle,
              onTap: () => unawaited(_import()),
            ),
          ],
        );
      },
    );
  }
}

/// Phones share files; desktops save them.
final bool _sharesFiles = Platform.isAndroid || Platform.isIOS;

const _backupFiles = XTypeGroup(
  label: 'Jitter backup',
  extensions: ['json'],
  mimeTypes: ['application/json'],
  uniformTypeIdentifiers: ['public.json'],
);

/// A save dialog on desktop; the share sheet on mobile, which has no
/// folder the app can write to for the user.
Future<String?> saveBackup(String fileName, String json) async {
  if (_sharesFiles) {
    final dir = await getTemporaryDirectory();
    final file = File('${dir.path}/$fileName');
    await file.writeAsString(json);
    await SharePlus.instance.share(
      ShareParams(files: [XFile(file.path, mimeType: 'application/json')]),
    );
    return null;
  }
  final location = await getSaveLocation(
    suggestedName: fileName,
    acceptedTypeGroups: const [_backupFiles],
  );
  if (location == null) return null;
  await File(location.path).writeAsString(json);
  return location.path;
}

/// The platform's open dialog.
Future<String?> pickBackup() async {
  final file = await openFile(acceptedTypeGroups: const [_backupFiles]);
  return file == null ? null : await file.readAsString();
}
