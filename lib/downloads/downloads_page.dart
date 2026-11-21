import 'dart:async';

import 'package:jitter/downloads/downloads_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:jitter/widgets/escape_goes_back.dart';
import 'package:material_ui/material_ui.dart';

/// The downloads as Rust last sent them. A running one can be cancelled;
/// any other can be removed from the list.
class DownloadsPage extends StatefulWidget {
  const new({required this.downloads, super.key});

  final DownloadsHolder downloads;

  /// Pushes the page. The holder already has the list, so nothing is asked.
  static Future<void> push(
    BuildContext context, {
    required DownloadsHolder downloads,
  }) {
    return Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (context) =>
            EscapeGoesBack(child: DownloadsPage(downloads: downloads)),
      ),
    );
  }

  @override
  State<DownloadsPage> createState() => _DownloadsPageState();
}

class _DownloadsPageState extends State<DownloadsPage> {
  late final StreamSubscription<String> _failures;

  @override
  void initState() {
    super.initState();
    _failures = widget.downloads.failures.listen(_showFailure);
  }

  void _showFailure(String message) => _showSnackBar(context, message);

  @override
  void dispose() {
    unawaited(_failures.cancel());
    super.dispose();
  }

  Widget _tile(DownloadItem item) {
    return ListTile(
      key: ValueKey(item.key),
      title: Text(item.fileName),
      subtitle: _subtitle(item),
      trailing: item.state is DownloadStateRunning
          ? IconButton(
              icon: const Icon(Icons.close),
              tooltip: 'Cancel',
              onPressed: () => widget.downloads.cancel(item.key),
            )
          : IconButton(
              icon: const Icon(Icons.delete_outline),
              tooltip: 'Remove',
              onPressed: () => widget.downloads.remove(item.key),
            ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: widget.downloads,
      builder: (context, _) {
        final downloads = widget.downloads.downloads;
        return Scaffold(
          appBar: AppBar(title: const Text('Downloads')),
          body: downloads.isEmpty
              ? const Center(
                  child: Text(
                    'No downloads yet. Long-press a VOD to download it.',
                  ),
                )
              : ListView(children: [for (final item in downloads) _tile(item)]),
        );
      },
    );
  }
}

Widget _subtitle(DownloadItem item) {
  return switch (item.state) {
    DownloadStateRunning() => Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text('${item.done} / ${item.total} segments · ${_mib(item.bytes)} MiB'),
        LinearProgressIndicator(
          value: item.total == 0 ? null : item.done / item.total,
        ),
      ],
    ),
    DownloadStateDone(:final path) => Text('Saved to $path'),
    DownloadStateFailed(:final message) => Text('Failed: $message'),
    // DownloadState is not sealed, so the cancelled state needs the default.
    _ => const Text('Cancelled'),
  };
}

/// [bytes] in MiB, to one decimal place.
String _mib(Uint64 bytes) => (bytes.toInt() / (1024 * 1024)).toStringAsFixed(1);

void _showSnackBar(BuildContext context, String message) {
  ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(message)));
}
