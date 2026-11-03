import 'package:jitter/src/bindings/bindings.dart';

/// Writes [message] from [source] (`flutter`, `mpv/ffmpeg`) to this run's
/// log, which Rust keeps. Sent before Rust has booted, it waits for it.
void logToRust(DartLogLevel level, String source, String message) =>
    LogFromDart(
      level: level,
      source: source,
      message: message,
    ).sendSignalToRust();
