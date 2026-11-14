import 'package:flutter_test/flutter_test.dart';

import '../../bench/process_load.dart';

/// typeperf output for three instances (two `3D` engines, one
/// `VideoDecode`) followed by [dataLines], then typeperf's trailing lines.
/// Windows line endings.
String _typeperfOutput(List<String> dataLines) => [
  _csvLine([
    '(PDH-CSV 4.0) (W. Europe Standard Time)(-60)',
    _counter(0, '3D'),
    _counter(1, '3D'),
    _counter(2, 'VideoDecode'),
  ]),
  ...dataLines,
  '',
  'Exiting, please wait...',
  'The command completed successfully.',
  '',
].join('\r\n');

/// Two data rows, with a blank cell and a `12,5` comma value.
///
/// Row 1: 3D = 10 + 5 = 15, VideoDecode = 3.
/// Row 2: 3D = blank + 12,5 = 12.5, VideoDecode = 4.
/// Averages over the two rows: 3D = (15 + 12.5) / 2 = 13.75,
/// VideoDecode = (3 + 4) / 2 = 3.5.
final String _typeperfSample = _typeperfOutput([
  _csvLine(['11/14/2026 10:00:01.123', '10.000000', '5.000000', '3.000000']),
  _csvLine(['11/14/2026 10:00:02.123', ' ', '12,5', '4.000000']),
]);

/// A typeperf CSV line: every cell quoted.
String _csvLine(List<String> cells) => cells.map((c) => '"$c"').join(',');

/// One GPU engine's counter header, as typeperf names it (unquoted).
String _counter(int engine, String type) {
  final instance =
      'pid_4242_luid_0x00000000_0x0000D1A7_phys_0_eng_${engine}_engtype_$type';
  return '\\\\HOST\\GPU Engine($instance)\\Utilization Percentage';
}

/// A typeperf CSV line: every cell quoted.
void main() {
  group('cpuTimeFromProcStat', () {
    test('reads utime and stime after the last parenthesis', () {
      // comm is "(my (odd) app)"; utime 250 and stime 150 at 100 ticks/s.
      const stat =
          '1234 (my (odd) app) S 1 1 1 0 -1 4194560 100 0 0 0 250 150 '
          '0 0 20 0 1';
      expect(cpuTimeFromProcStat(stat, 100), const Duration(seconds: 4));
    });

    test('scales by the clock tick rate', () {
      const stat = '7 (x) R 0 0 0 0 0 0 0 0 0 0 50 50 0 0';
      expect(cpuTimeFromProcStat(stat, 50), const Duration(seconds: 2));
    });

    test('is null on a line that does not parse', () {
      expect(cpuTimeFromProcStat('garbage', 100), isNull);
      expect(cpuTimeFromProcStat('1 (a) S 0 0', 100), isNull);
      expect(
        cpuTimeFromProcStat('1 (a) S 0 0 0 0 0 0 0 0 0 0 x 1', 100),
        isNull,
      );
      expect(cpuTimeFromProcStat('1 (a) S 0 0 0 0 0 0 0 0 0 0 1 1', 0), isNull);
    });
  });

  group('gpuByEngineType', () {
    test('sums the instances of each engine and averages the rows', () {
      expect(gpuByEngineType(_typeperfSample), {
        '3D': 13.75,
        'VideoDecode': 3.5,
      });
    });

    test('leaves a row with no numeric cell out of the average', () {
      // Row 1 is blank in every cell, as a rate counter's first sample is.
      // Rows 2 and 3 count, so:
      //   3D: row 2 = 10 + 5 = 15, row 3 = 2.5 + 2.5 = 5, (15 + 5) / 2 = 10.
      //   VideoDecode: row 2 = 3, row 3 = 6, (3 + 6) / 2 = 4.5.
      // Counting row 1 as 0 would give 3D = 20 / 3 and VideoDecode = 9 / 3.
      final output = _typeperfOutput([
        _csvLine(['11/14/2026 10:00:01.123', ' ', ' ', ' ']),
        _csvLine([
          '11/14/2026 10:00:02.123',
          '10.000000',
          '5.000000',
          '3.000000',
        ]),
        _csvLine([
          '11/14/2026 10:00:03.123',
          '2.500000',
          '2.500000',
          '6.000000',
        ]),
      ]);
      expect(gpuByEngineType(output), {'3D': 10.0, 'VideoDecode': 4.5});
    });

    test('leaves an engine type out when no row has a numeric cell', () {
      // VideoDecode is blank in both rows, so it has no counted row. 3D:
      // row 1 = 10 + 5 = 15, row 2 = 1 + 2 = 3, (15 + 3) / 2 = 9.
      final output = _typeperfOutput([
        _csvLine(['11/14/2026 10:00:01.123', '10.000000', '5.000000', ' ']),
        _csvLine(['11/14/2026 10:00:02.123', '1.000000', '2.000000', ' ']),
      ]);
      expect(gpuByEngineType(output), {'3D': 9.0});
    });

    test('is empty with no data row', () {
      final headerOnly = _typeperfSample.split('\r\n').first;
      expect(gpuByEngineType(headerOnly), isEmpty);
    });

    test('is empty for typeperf error output', () {
      expect(gpuByEngineType('Error: No valid counters.\r\n'), isEmpty);
    });
  });

  group('typeperfProblem', () {
    test('is the first line that is neither CSV nor a routine message', () {
      expect(
        typeperfProblem('Error: No valid counters.\r\n'),
        'Error: No valid counters.',
      );
    });

    test('ignores CSV and the routine trailing lines', () {
      expect(typeperfProblem(_typeperfSample), isNull);
    });
  });

  group('LoadReport.toFields', () {
    test('prints every field, engine types sorted by name', () {
      const report = LoadReport(
        cpuPct: 42.34,
        cores: 8,
        rssAvgMb: 512,
        rssMaxMb: 600,
        gpu: {'VideoDecode': 3, '3D': 12.5},
      );
      expect(
        report.toFields(),
        ' cpu_pct=42.3 cores=8 rss_avg_mb=512 rss_max_mb=600 '
        'gpu=3D:12.5,VideoDecode:3.0',
      );
    });

    test('prints ? for every unknown field', () {
      const report = LoadReport();
      expect(
        report.toFields(),
        ' cpu_pct=? cores=? rss_avg_mb=? rss_max_mb=? gpu=?',
      );
    });

    test('prints the reason when GPU failed', () {
      const report = LoadReport(gpuProblem: 'Error: No valid counters.');
      expect(
        report.toFields(),
        ' cpu_pct=? cores=? rss_avg_mb=? rss_max_mb=? '
        'gpu=?(Error: No valid counters.)',
      );
    });
  });
}
