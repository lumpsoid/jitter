import 'package:jitter/src/bindings/bindings.dart';

/// A download row as Rust sends it; [key] tells the rows apart.
DownloadItem download({
  required String fileName,
  int key = 1,
  DownloadState state = const DownloadStateRunning(),
}) => DownloadItem(
  key: Uint64.parse('$key'),
  videoId: '42',
  fileName: fileName,
  done: 0,
  total: 0,
  bytes: Uint64.parse('0'),
  state: state,
);
