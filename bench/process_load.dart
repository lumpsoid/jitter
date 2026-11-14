import 'dart:async';
import 'dart:convert';
import 'dart:io';

/// The load of this bench process over a measured window: CPU, memory and,
/// on Windows, GPU engine use.
///
/// Nothing here adds a package or an FFI binding. CPU comes from
/// `/proc/self/stat` on Linux and from PowerShell on Windows; GPU comes from
/// `typeperf`. Every external read can fail, and a failure reads as `?`, so
/// nothing here throws.
///
/// typeperf's counter names are localized: on a non-English Windows the GPU
/// counter is not found, and the report says `gpu=?(<reason>)`.

const int _bytesPerMib = 1024 * 1024;
const _defaultTicksPerSecond = 100;
const _gpuTimeout = Duration(seconds: 10);
const _noGpuRows = 'no GPU engine rows';
final Converter<List<int>, String> _decoder = const Utf8Codec(
  allowMalformed: true,
).decoder;
final RegExp _cellPattern = RegExp('"([^"]*)"');

/// CPU time (user plus system) from one `/proc/<pid>/stat` line, or null when
/// the line does not parse.
Duration? cpuTimeFromProcStat(String stat, int ticksPerSecond) {
  if (ticksPerSecond <= 0) return null;
  // comm, the second field, is in parentheses and may hold spaces and ')',
  // so the fields are counted from the last ')'.
  final close = stat.lastIndexOf(')');
  if (close < 0) return null;
  final fields = stat.substring(close + 1).trim().split(RegExp(r'\s+'));
  // After the ')', `state` is field 3 of the line, so utime (field 14) and
  // stime (field 15) are at indexes 11 and 12.
  if (fields.length < 13) return null;
  final utime = int.tryParse(fields[11]);
  final stime = int.tryParse(fields[12]);
  if (utime == null || stime == null) return null;
  final ticks = utime + stime;
  return Duration(microseconds: ticks * 1000000 ~/ ticksPerSecond);
}

/// GPU utilization percent per engine type (`3D`, `VideoDecode`, ...) from
/// typeperf's CSV output. For each engine type, a data row counts only when
/// at least one of that type's cells in the row is numeric; the row's value is
/// the sum of its numeric cells. The counted rows are averaged, so a blank
/// first sample of a rate counter does not drag the average down. An engine
/// type with no counted row is left out. Empty when there is no header or no
/// data row.
Map<String, double> gpuByEngineType(String typeperfOutput) {
  final rows = _csvRows(typeperfOutput);
  if (rows.length < 2) return const {};
  final engineOf = <int, String>{};
  for (var column = 0; column < rows.first.length; column++) {
    final engine = _engineType(rows.first[column]);
    if (engine != null) engineOf[column] = engine;
  }
  final data = rows.sublist(1);
  final result = <String, double>{};
  for (final engine in engineOf.values.toSet()) {
    var total = 0.0;
    var counted = 0;
    for (final row in data) {
      final sum = _rowSum(row, engineOf, engine);
      if (sum == null) continue;
      total += sum;
      counted++;
    }
    if (counted > 0) result[engine] = total / counted;
  }
  return result;
}

/// The first line of typeperf's output that is not CSV and not one of its
/// routine messages, or null when there is none.
String? typeperfProblem(String typeperfOutput) {
  for (final raw in const LineSplitter().convert(typeperfOutput)) {
    final line = raw.trim();
    if (line.isEmpty || line.startsWith('"')) continue;
    if (line == 'Exiting, please wait...') continue;
    if (line == 'The command completed successfully.') continue;
    return line;
  }
  return null;
}

/// One window's load, as one line of fields. A null field is unknown and
/// prints as `?`.
class LoadReport {
  /// Creates a report; every field is unknown unless given.
  const new({
    this.cpuPct,
    this.cores,
    this.rssAvgMb,
    this.rssMaxMb,
    this.gpu,
    this.gpuProblem,
  });

  /// CPU time as a percent of one core, so it can exceed 100.
  final double? cpuPct;

  /// The number of logical processors.
  final int? cores;

  /// Average and maximum resident memory over the window, in whole MB.
  final int? rssAvgMb;
  final int? rssMaxMb;

  /// Percent per engine type. Null where GPU is not measured (not Windows).
  final Map<String, double>? gpu;

  /// Why [gpu] is unknown, when it was attempted and failed.
  final String? gpuProblem;

  /// One line of space-led `key=value` fields.
  String toFields() =>
      ' cpu_pct=${_fixed(cpuPct)} cores=${_whole(cores)}'
      ' rss_avg_mb=${_whole(rssAvgMb)} rss_max_mb=${_whole(rssMaxMb)}'
      ' gpu=${_gpuField()}';

  String _gpuField() {
    final engines = gpu;
    if (engines != null && engines.isNotEmpty) {
      final names = engines.keys.toList()..sort();
      return [for (final name in names) '$name:${_fixed(engines[name])}']
          .join(',');
    }
    final problem = gpuProblem;
    return problem == null ? '?' : '?($problem)';
  }

  static String _fixed(double? value) =>
      value == null ? '?' : value.toStringAsFixed(1);

  static String _whole(int? value) => value == null ? '?' : '$value';
}

/// Measures this process over one window: [start] when it begins and [stop]
/// when it ends. Neither throws; a read that fails leaves its field unknown.
class LoadSampler {
  final List<int> _rssBytes = [];
  final Stopwatch _wall = Stopwatch();
  Timer? _rssTimer;
  int _ticksPerSecond = _defaultTicksPerSecond;
  Duration? _cpuAtStart;
  Process? _typeperf;
  Future<String>? _gpuOutput;

  /// Starts the window. [window] is how long the GPU counter runs.
  Future<void> start(Duration window) async {
    if (Platform.isLinux) _ticksPerSecond = await _clockTicks();
    _cpuAtStart = await _cpuTime();
    _wall.start();
    _sampleRss();
    _rssTimer = Timer.periodic(const Duration(seconds: 1), (_) => _sampleRss());
    if (Platform.isWindows) await _startGpu(window);
  }

  /// Ends the window and reports what it measured.
  Future<LoadReport> stop() async {
    _rssTimer?.cancel();
    final cpuAtEnd = await _cpuTime();
    _wall.stop();
    final output = await _finishGpu();
    final gpu = output == null ? null : gpuByEngineType(output);
    final problem = output == null ? null : typeperfProblem(output);
    final samples = _rssBytes;
    final avgBytes = samples.isEmpty
        ? null
        : samples.fold<int>(0, (sum, bytes) => sum + bytes) / samples.length;
    final maxBytes = samples.isEmpty
        ? null
        : samples.reduce((a, b) => a > b ? a : b);
    return LoadReport(
      cpuPct: _cpuPercent(_cpuAtStart, cpuAtEnd, _wall.elapsed),
      cores: Platform.numberOfProcessors,
      rssAvgMb: avgBytes == null ? null : _toMib(avgBytes),
      rssMaxMb: maxBytes == null ? null : _toMib(maxBytes),
      gpu: gpu,
      gpuProblem: gpu != null && gpu.isEmpty && problem == null
          ? _noGpuRows
          : problem,
    );
  }

  void _sampleRss() => _rssBytes.add(ProcessInfo.currentRss);

  Future<int> _clockTicks() async {
    try {
      final result = await Process.run('getconf', ['CLK_TCK']);
      final ticks = int.tryParse('${result.stdout}'.trim());
      return ticks != null && ticks > 0 ? ticks : _defaultTicksPerSecond;
    } on Exception {
      return _defaultTicksPerSecond;
    }
  }

  Future<Duration?> _cpuTime() async {
    try {
      if (Platform.isLinux) {
        final stat = await File('/proc/self/stat').readAsString();
        return cpuTimeFromProcStat(stat, _ticksPerSecond);
      }
      if (Platform.isWindows) {
        final result = await Process.run('powershell', [
          '-NoProfile',
          '-Command',
          '(Get-Process -Id $pid).TotalProcessorTime.TotalMilliseconds',
        ]);
        final text = '${result.stdout}'.trim().replaceAll(',', '.');
        final ms = double.tryParse(text);
        return ms == null ? null : Duration(microseconds: (ms * 1000).round());
      }
      return null;
    } on Exception {
      return null;
    }
  }

  Future<void> _startGpu(Duration window) async {
    final seconds = window.inSeconds < 1 ? 1 : window.inSeconds;
    final counter = '\\GPU Engine(pid_${pid}_*)\\Utilization Percentage';
    try {
      final proc = await Process.start('typeperf', [
        counter,
        '-si',
        '1',
        '-sc',
        '$seconds',
      ]);
      _typeperf = proc;
      _gpuOutput = _readAll(proc);
    } on Exception catch (e) {
      _gpuOutput = Future.value('$e');
    }
  }

  /// typeperf's output once it has exited (killed after [_gpuTimeout]), or
  /// null when GPU is not measured here.
  Future<String?> _finishGpu() async {
    final output = _gpuOutput;
    if (output == null) return null;
    final proc = _typeperf;
    try {
      if (proc != null) {
        await proc.exitCode.timeout(
          _gpuTimeout,
          onTimeout: () {
            proc.kill();
            return -1;
          },
        );
      }
      return await output;
    } on Exception catch (e) {
      return '$e';
    }
  }

  static Future<String> _readAll(Process proc) async {
    final out = proc.stdout.transform(_decoder).join();
    final err = proc.stderr.transform(_decoder).join();
    return '${await out}\n${await err}';
  }

  static double? _cpuPercent(Duration? from, Duration? to, Duration wall) {
    if (from == null || to == null || wall <= Duration.zero) return null;
    return (to - from).inMicroseconds / wall.inMicroseconds * 100;
  }

  static int _toMib(num bytes) => (bytes / _bytesPerMib).round();
}

List<List<String>> _csvRows(String output) => [
  for (final line in const LineSplitter().convert(output))
    if (line.startsWith('"')) _cells(line),
];

List<String> _cells(String line) => _cellPattern
    .allMatches(line)
    .map((match) => match.group(1))
    .whereType<String>()
    .toList();

/// The engine type of a counter header cell, such as `3D` for
/// `...(pid_4242_..._engtype_3D)\Utilization Percentage`.
String? _engineType(String cell) {
  const key = 'engtype_';
  final start = cell.indexOf(key);
  if (start < 0) return null;
  final from = start + key.length;
  final end = cell.indexOf(')', from);
  if (end <= from) return null;
  return cell.substring(from, end);
}

/// The value of one data cell, or null when it is blank or not a number.
/// A `,` decimal separator (non-English Windows) is read as `.`.
double? _cellValue(String cell) {
  final text = cell.trim();
  if (text.isEmpty) return null;
  return double.tryParse(text.replaceAll(',', '.'));
}

/// The sum of the numeric cells of [engine] in [row], or null when none of
/// them is numeric.
double? _rowSum(List<String> row, Map<int, String> engineOf, String engine) {
  double? sum;
  for (final column in engineOf.keys) {
    if (engineOf[column] != engine || column >= row.length) continue;
    final value = _cellValue(row[column]);
    if (value != null) sum = (sum ?? 0.0) + value;
  }
  return sum;
}
