import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:jitter/player/media_kit_engine.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:material_ui/material_ui.dart';
import 'package:rinf/rinf.dart';

import 'bench_run.dart';
import 'bench_screens.dart';
import 'resolver.dart';
import 'subjects.dart';
import 'targets.dart';

/// The build the log header names. CI passes the commit with `--dart-define`.
const _build = String.fromEnvironment(
  'JITTER_BENCH_BUILD',
  defaultValue: 'local',
);

/// The channel media_kit's patched Windows video output logs on.
const _videoLog = MethodChannel('jitter/media_kit_video/log');

/// libmdk lines to keep: its warnings and severe errors, and any whose
/// message mentions a decoder. Lines are `MDK <level> <logger>: <message>`.
final _mdkKept = RegExp(
  '^MDK (WARNING|SEVERE) |^MDK [^:]*: .*decoder',
  caseSensitive: false,
  dotAll: true,
);

/// The GUI bench, for a tester on Windows: type what to play, pick an
/// engine, press Run test, then copy the log back.
///
/// `JITTER_BENCH_TARGETS`, `JITTER_BENCH_ENGINE`, `JITTER_BENCH_WARMUP` and
/// `JITTER_BENCH_SECONDS` pre-fill the setup fields. With
/// `JITTER_BENCH_AUTORUN=1` the first frame presses Run, and the app exits
/// when the run ends. Those are for local checks.
Future<void> main() async {
  final started = Stopwatch()..start();
  WidgetsFlutterBinding.ensureInitialized();
  await initializeRust(assignRustSignal);
  runApp(
    MaterialApp(
      debugShowCheckedModeBanner: false,
      theme: ThemeData.dark(),
      home: BenchHome(started: started),
    ),
  );
}

/// Which screen the bench shows. `resolving` is the setup screen while the
/// targets resolve, with the form frozen.
enum _Phase { setup, resolving, playing, result }

/// The bench's one screen holder. Its state is the setup form, the phase, the
/// engine under test, and the log, which holds every run of this launch.
class BenchHome extends StatefulWidget {
  const new({required this.started, super.key});

  /// Taken at launch: the log's time stamps count from it.
  final Stopwatch started;

  @override
  State<BenchHome> createState() => _BenchHomeState();
}

class _BenchHomeState extends State<BenchHome> {
  final Map<String, String> _env = Platform.environment;
  late final _targets = TextEditingController(
    text: _env['JITTER_BENCH_TARGETS'] ?? '',
  );
  final _hwdec = TextEditingController();
  late final _warmup = TextEditingController(
    text: _env['JITTER_BENCH_WARMUP'] ?? '10',
  );
  late final _measured = TextEditingController(
    text: _env['JITTER_BENCH_SECONDS'] ?? '30',
  );
  late String _engine = _initialEngine();
  final _lines = <String>[];

  /// The lifecycle states seen since the last [_visibility] call.
  final _seen = <String>{};
  late final AppLifecycleListener _lifecycle;
  _Phase _phase = _Phase.setup;
  BenchSubject? _subject;
  String _status = '';

  bool get _autorun => _env['JITTER_BENCH_AUTORUN'] == '1';

  @override
  void initState() {
    super.initState();
    _lifecycle = AppLifecycleListener(
      onStateChange: (state) => _seen.add(state.name),
    );
    if (_autorun) {
      WidgetsBinding.instance.addPostFrameCallback((_) => unawaited(_run()));
    }
  }

  @override
  void dispose() {
    _lifecycle.dispose();
    _targets.dispose();
    _hwdec.dispose();
    _warmup.dispose();
    _measured.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return switch (_phase) {
      _Phase.setup || _Phase.resolving => SetupScreen(
        buildId: _build,
        targets: _targets,
        hwdec: _hwdec,
        warmup: _warmup,
        measured: _measured,
        engine: _engine,
        locked: _subject != null,
        running: _phase == _Phase.resolving,
        canRun:
            _phase == _Phase.setup && parseTargets(_targets.text).isNotEmpty,
        status: _status,
        hasLog: _lines.isNotEmpty,
        onEngine: (engine) => _update(() => _engine = engine),
        onTargetsChanged: (_) => _update(() {}),
        onRun: _run,
        onShowLog: () => _update(() => _phase = _Phase.result),
      ),
      _Phase.playing => PlayingScreen(
        picture: _subject?.picture,
        status: _status,
      ),
      _Phase.result => ResultScreen(
        log: _lines.join('\n'),
        onCopy: _copy,
        onBack: () => _update(() => _phase = _Phase.setup),
      ),
    };
  }

  /// One test: resolves the targets, opens the engine on the first run, plays
  /// the streams, then shows the result.
  Future<void> _run() async {
    final targets = parseTargets(_targets.text);
    if (targets.isEmpty) return;
    _update(() {
      _phase = _Phase.resolving;
      _status = 'Resolving...';
    });
    _header();
    final warmup = int.tryParse(_warmup.text);
    final measured = int.tryParse(_measured.text);
    if (warmup == null || measured == null) {
      _fail('settings', 'warm-up and measured must be whole seconds');
      return;
    }
    final streams = await _resolve(targets);
    if (streams == null) return;
    final subject = _subjectFor(_hwdec.text.trim());
    if (subject == null) return;
    _update(() {
      _phase = _Phase.playing;
      _status = 'Starting...';
    });
    await BenchRun(
      subject: subject,
      streams: streams,
      warmup: Duration(seconds: warmup),
      window: Duration(seconds: measured),
      log: _add,
      status: _setStatus,
      visibility: _visibility,
    ).run();
    _update(() => _phase = _Phase.result);
    if (_autorun) exit(0);
  }

  /// Each target's stream URL, in the order the runs play them. Logs the
  /// failure and returns `null` when one does not resolve.
  Future<List<(BenchTarget, String)>?> _resolve(
    List<BenchTarget> targets,
  ) async {
    try {
      if (targets.any((target) => target is! UrlTarget)) await bootHub();
      final streams = <(BenchTarget, String)>[];
      for (final target in runOrder(targets)) {
        final url = await resolveUrl(target);
        // A full URL can carry a token, so only its host is logged.
        _add('RESOLVED ${target.label} host=${Uri.parse(url).host}');
        streams.add((target, url));
      }
      return streams;
    } on Object catch (error) {
      _fail('resolve', error);
      return null;
    }
  }

  /// The engine under test. The first run creates it; later runs reuse it,
  /// since one launch tests one engine. Logs the failure and returns `null`
  /// when it cannot be created.
  BenchSubject? _subjectFor(String hwdec) {
    final existing = _subject;
    if (existing != null) return existing;
    try {
      final subject = benchSubject(
        _engine,
        hwdec: hwdec.isEmpty ? null : hwdec,
        log: _fromSubject,
      );
      if (subject.engine case final MediaKitEngine mpv) _takeVideoLog(mpv);
      _update(() => _subject = subject);
      return subject;
    } on Object catch (error) {
      _fail('engine', error);
      return null;
    }
  }

  /// Takes media_kit's video-output log, which the Windows device checks
  /// answer on, so its lines land in this log. Reading the controller first
  /// creates the player, and that installs the app's own handler. A channel
  /// has one handler, so this one must come after.
  void _takeVideoLog(MediaKitEngine engine) {
    final _ = engine.controller;
    _videoLog.setMethodCallHandler(_onVideoLog);
  }

  /// media_kit's video-output lines, such as the D3D 11 device state.
  Future<void> _onVideoLog(MethodCall call) async {
    if (call.arguments case {
      'level': final String level,
      'message': final String message,
    }) {
      _add('VIDEO $level $message');
    }
  }

  /// The run's first lines: the build, the machine, and the window.
  void _header() {
    final view = View.of(context);
    final size = view.physicalSize / view.devicePixelRatio;
    _add(
      'BENCH build=$_build os=${Platform.operatingSystem}'
      ' "${Platform.operatingSystemVersion}"'
      ' cores=${Platform.numberOfProcessors} mode=${_buildMode()}',
    );
    _add(
      'WINDOW ${size.width.round()}x${size.height.round()}'
      ' dpr=${view.devicePixelRatio}',
    );
  }

  /// A subject's line. libmdk's lines are kept only when they matter.
  void _fromSubject(String line) {
    if (!line.startsWith('MDK ') || _mdkKept.hasMatch(line)) _add(line);
  }

  /// Logs [line], stamped with the seconds since launch, and writes it to
  /// stdout too.
  void _add(String line) {
    final seconds =
        widget.started.elapsedMicroseconds / Duration.microsecondsPerSecond;
    final text = '[${seconds.toStringAsFixed(1).padLeft(6)}] $line';
    stdout.writeln(text);
    _update(() => _lines.add(text));
  }

  /// Logs a failure, then shows the result. Under autorun the app exits: 1
  /// here, 0 after a run that finished.
  void _fail(String where, Object error) {
    _add('ERROR $where: $error');
    _update(() => _phase = _Phase.result);
    if (_autorun) exit(1);
  }

  void _setStatus(String text) => _update(() => _status = text);

  /// The lifecycle state now, then the states seen since the last call, each
  /// once. Clears what was seen.
  String _visibility() {
    final now = WidgetsBinding.instance.lifecycleState?.name ?? 'unknown';
    final names = {now, ..._seen};
    _seen.clear();
    return names.join(',');
  }

  Future<void> _copy() async {
    await Clipboard.setData(ClipboardData(text: _lines.join('\n')));
    if (mounted) {
      ScaffoldMessenger.of(context)
          .showSnackBar(const SnackBar(content: Text('Log copied')));
    }
  }

  /// Applies [change] and rebuilds, if the screen is still mounted.
  void _update(VoidCallback change) {
    if (mounted) {
      setState(change);
    } else {
      change();
    }
  }

  /// The engine the environment asks for. exo runs only on Android; anything
  /// else is mpv.
  String _initialEngine() => switch (_env['JITTER_BENCH_ENGINE']) {
    'fvp' => 'fvp',
    'exo' when Platform.isAndroid => 'exo',
    _ => 'mpv',
  };
}

/// Which Flutter build this is: `release`, `profile` or `debug`.
String _buildMode() {
  if (kReleaseMode) return 'release';
  return kProfileMode ? 'profile' : 'debug';
}
