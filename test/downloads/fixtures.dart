import 'package:jitter/src/bindings/bindings.dart';

/// A download row as Rust sends it; [key] tells the rows apart.
DownloadItem download({
  required String fileName,
  int key = 1,
  DownloadState state = const DownloadStateRunning(),
  int done = 0,
  int total = 0,
  int bytes = 0,
}) => DownloadItem(
  key: Uint64.parse('$key'),
  videoId: '42',
  fileName: fileName,
  done: done,
  total: total,
  bytes: Uint64.parse('$bytes'),
  state: state,
);
