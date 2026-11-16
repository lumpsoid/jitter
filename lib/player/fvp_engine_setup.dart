// Temporary engine pick: the video_player engine (fvp/libmdk on Linux and
// Windows) is checked against media_kit on devices. This file goes when
// media_kit is removed.

import 'package:fvp/fvp.dart' as fvp;
import 'package:fvp/fvp.dart' show FVPControllerExtensions;
import 'package:jitter/logs/rust_log.dart';
import 'package:jitter/player/player_engine.dart';
import 'package:jitter/player/video_player_engine.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:logging/logging.dart';

/// Whether the [environment] picks the fvp engine: `JITTER_ENGINE=fvp`.
/// Anything else, or no variable, keeps media_kit.
bool picksFvp(Map<String, String> environment) =>
    environment['JITTER_ENGINE'] == 'fvp';

/// The Rust log level for an fvp or MDK record at [level]. Rust keeps the
/// verbose level only for mpv's lines, so everything below a warning goes
/// as info.
DartLogLevel rustLevelFor(Level level) {
  if (level >= Level.SEVERE) return DartLogLevel.error;
  if (level >= Level.WARNING) return DartLogLevel.warning;
  return DartLogLevel.info;
}

/// Loggers whose records go to the Rust log: fvp's own and the MDK's.
const _forwardedLoggers = {'mdk', 'fvp'};

/// Registers fvp for Linux and Windows, forwards its logs to the Rust log,
/// and returns a [VideoPlayerEngine] on it. Call once, before the engine is
/// used.
PlayerEngine startFvpEngine() {
  Logger.root.level = Level.ALL;
  Logger.root.onRecord
      .where((record) => _forwardedLoggers.contains(record.loggerName))
      .listen(
        (record) => logToRust(
          rustLevelFor(record.level),
          record.loggerName,
          record.message,
        ),
      );
  fvp.registerWith(
    options: {
      'platforms': ['linux', 'windows'],
    },
  );
  return VideoPlayerEngine(
    stepFrames: (controller, frames) => controller.step(frames: frames),
  );
}
