import 'dart:async';
import 'dart:io';

import 'package:jitter/downloads/downloads_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:material_ui/material_ui.dart';
import 'package:path_provider/path_provider.dart';

/// Downloads [videoId] from the GUI: asks Rust for its qualities, lets the
/// user pick one in a sheet, then starts it into [downloadDir]. Reports a
/// failure or the outcome in a snackbar; [onView] adds a View action to the
/// started download's snackbar.
Future<void> downloadVod(
  BuildContext context, {
  required DownloadsHolder downloads,
  required String videoId,
  Future<String> Function() downloadDir = platformDownloadDir,
  VoidCallback? onView,
}) async {
  final answer = await downloads.choose(videoId);
  if (!context.mounted) return;
  if (answer is DownloadsUpdateFailed) {
    _showSnackBar(context, answer.message);
    return;
  }
  final qualities = (answer as DownloadsUpdateChoices).qualities;
  if (qualities.isEmpty) {
    _showSnackBar(context, 'This VOD has no qualities to download.');
    return;
  }
  final name = await showModalBottomSheet<String>(
    context: context,
    builder: (sheetContext) => SafeArea(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const ListTile(title: Text('Download in quality')),
          for (final q in qualities)
            ListTile(
              title: Text(q.label),
              onTap: () => Navigator.pop(sheetContext, q.name),
            ),
        ],
      ),
    ),
  );
  if (name == null || !context.mounted) return;
  final String dir;
  try {
    dir = await downloadDir();
  } on Object catch (e) {
    if (context.mounted) {
      _showSnackBar(context, 'No folder to save into: $e');
    }
    return;
  }
  downloads.start(videoId: videoId, quality: name, dir: dir);
  if (!context.mounted) return;
  ScaffoldMessenger.of(context).showSnackBar(
    SnackBar(
      content: const Text('Download started'),
      action: onView == null
          ? null
          : SnackBarAction(label: 'View', onPressed: onView),
    ),
  );
}

/// The folder downloads save into. Android gets the app's own external
/// folder and iOS its documents folder, both writable without a storage
/// permission; elsewhere the system Downloads folder, or the documents
/// folder when there is none.
Future<String> platformDownloadDir() async {
  final Directory? dir;
  if (Platform.isAndroid) {
    dir = await getExternalStorageDirectory();
  } else if (Platform.isIOS) {
    dir = await getApplicationDocumentsDirectory();
  } else {
    dir = await getDownloadsDirectory();
  }
  return (dir ?? await getApplicationDocumentsDirectory()).path;
}

void _showSnackBar(BuildContext context, String message) {
  ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(message)));
}
