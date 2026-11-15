import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:rinf/rinf.dart';

import 'subjects.dart';

/// Plays `JITTER_BENCH_URL` full-window with the engine named by
/// `JITTER_BENCH_ENGINE` (`mpv`, `fvp`, or `exo` on Android), as a bare
/// picture with no controls, then reports what the engine did over the
/// measured window.
///
/// Both engines run in this one build, so the system load is comparable.
/// `exo` is video_player's own ExoPlayer, which runs when fvp is not
/// registered. Output: `BENCH_START engine=<name>` when measuring begins,
/// then one `BENCH_END ...` line when it ends. Failures are `BENCH_ERROR`
/// (on stderr, except on Android).
///
/// Android apps get no environment from the host, so there the settings
/// come from a socket on 127.0.0.1:8765 as `KEY=VALUE` lines, and output
/// goes to logcat, where stdout is not visible.
///
/// `JITTER_BENCH_MPV_HWDEC`, when set, is the mpv `hwdec` the mpv engine
/// decodes with. It is set before the URL opens and echoed as
/// `BENCH_MPV_HWDEC <value>`.
///
/// Flutter frame timings are not counted: the Linux embedder redraws video
/// textures without a Dart frame, so they never show up as `FrameTiming`s.
Future<void> main() async {
  final env = await _config();
  final url = env['JITTER_BENCH_URL'];
  if (url == null || url.isEmpty) _fail(2, 'no JITTER_BENCH_URL');
  final name = env['JITTER_BENCH_ENGINE'] ?? 'mpv';
  if (name != 'mpv' && name != 'fvp' && name != 'exo') {
    _fail(2, 'unknown JITTER_BENCH_ENGINE $name');
  }
  if (name == 'exo' && !Platform.isAndroid) {
    _fail(2, 'JITTER_BENCH_ENGINE exo runs only on Android');
  }
  final warmup = _seconds(env, 'JITTER_BENCH_WARMUP', 15);
  final seconds = _seconds(env, 'JITTER_BENCH_SECONDS', 60);

  WidgetsFlutterBinding.ensureInitialized();
  await initializeRust(assignRustSignal);
  final subject = benchSubject(
    name,
    decoders: _decoders(env),
    hwdec: _mpvHwdec(env),
    log: _emit,
  );
  runApp(
    MaterialApp(
      debugShowCheckedModeBanner: false,
      home: ColoredBox(
        color: Colors.black,
        child: Center(child: subject.picture),
      ),
    ),
  );
  unawaited(
    _measure(
      subject,
      url: url,
      warmup: Duration(seconds: warmup),
      window: Duration(seconds: seconds),
    ),
  );
}

/// The port the sampler serves the Android settings on, reached through
/// `adb reverse tcp:8765 tcp:8765`.
const _configPort = 8765;

/// The bench's settings. Desktop reads the environment. Android gets none
/// from the host, so it reads `KEY=VALUE` lines from [_configPort], which
/// the sampler closes after writing them.
Future<Map<String, String>> _config() async {
  if (!Platform.isAndroid) return Platform.environment;
  try {
    final socket = await Socket.connect(
      '127.0.0.1',
      _configPort,
      timeout: const Duration(seconds: 10),
    );
    return _parseConfig(await utf8.decodeStream(socket));
  } on Object catch (error) {
    _fail(2, 'no config on 127.0.0.1:$_configPort: $error');
  }
}

/// The `KEY=VALUE` lines in [text] as a map, each split at its first `=`.
/// Lines without an `=`, such as blank ones, are skipped.
Map<String, String> _parseConfig(String text) {
  final config = <String, String>{};
  for (final line in const LineSplitter().convert(text)) {
    final at = line.indexOf('=');
    if (at < 0) continue;
    config[line.substring(0, at).trim()] = line.substring(at + 1).trim();
  }
  return config;
}

/// Writes one bench line where this platform shows it: logcat on Android,
/// and stdout or stderr on desktop. `debugPrintSynchronously` is `print`
/// without the throttling of `debugPrint`, which would delay the markers.
void _emit(String line, {bool error = false}) {
  if (Platform.isAndroid) {
    debugPrintSynchronously(line);
  } else if (error) {
    stderr.writeln(line);
  } else {
    stdout.writeln(line);
  }
}

/// Reports a usage error as `BENCH_ERROR` and exits with [code].
Never _fail(int code, String message) {
  _emit('BENCH_ERROR $message', error: true);
  exit(code);
}

/// The whole number of seconds in the variable [name], or [fallback] when
/// it is unset.
int _seconds(Map<String, String> env, String name, int fallback) {
  final raw = env[name];
  if (raw == null) return fallback;
  return int.tryParse(raw) ?? _fail(2, 'bad $name $raw');
}

/// The decoder names in `JITTER_BENCH_FVP_DECODERS` (comma-separated), or
/// `null` when it is unset or empty, which keeps fvp's own list.
List<String>? _decoders(Map<String, String> env) {
  final names = (env['JITTER_BENCH_FVP_DECODERS'] ?? '')
      .split(',')
      .map((name) => name.trim())
      .where((name) => name.isNotEmpty)
      .toList();
  return names.isEmpty ? null : names;
}

/// The mpv `hwdec` in `JITTER_BENCH_MPV_HWDEC`, or `null` when it is unset
/// or empty, which keeps the player's own choice.
String? _mpvHwdec(Map<String, String> env) {
  final value = env['JITTER_BENCH_MPV_HWDEC'];
  return value == null || value.isEmpty ? null : value;
}

/// Opens [url] at zero and lets it warm up, then times the [window] and
/// ends the run.
Future<void> _measure(
  BenchSubject subject, {
  required String url,
  required Duration warmup,
  required Duration window,
}) async {
  try {
    await subject.prepare();
    await subject.engine.open(url, start: Duration.zero);
    await Future<void>.delayed(warmup);
    await subject.startWindow();
    final clock = Stopwatch()..start();
    _emit('BENCH_START engine=${subject.name}');
    await Future<void>.delayed(window);
    clock.stop();
    final fields = await subject.endWindow();
    final seconds = clock.elapsedMilliseconds / 1000;
    _emit(
      'BENCH_END engine=${subject.name}'
      ' seconds=${seconds.toStringAsFixed(1)}'
      ' position_s=${subject.engine.position.inSeconds}$fields',
    );
    await subject.engine.stop();
    exit(0);
  } on Object catch (error) {
    _emit('BENCH_ERROR $error', error: true);
    exit(1);
  }
}
