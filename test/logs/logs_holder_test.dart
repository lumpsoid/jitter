import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/logs/logs_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';

void main() {
  late List<Object> sent;
  late LogsHolder holder;

  setUp(() {
    sent = [];
    holder = LogsHolder(sendLogsReady: sent.add, sendOpenLog: sent.add);
  });

  test('refresh asks for the sessions', () {
    holder.refresh();

    expect(sent.single, isA<LogsReady>());
  });

  test('a list replaces the last', () {
    const list = LogsUpdateSessions(
      dir: '/data/jitter/logs',
      sessions: [
        LogSession(
          name: '2026-11-03_14-22-05.log',
          label: '2026-11-03 14:22:05',
          current: true,
        ),
      ],
    );

    holder.apply(list);

    expect(holder.sessions, same(list));
  });

  test('opening a session forgets the last until its log arrives', () {
    holder
      ..apply(const LogsUpdateOpened(name: 'a.log', path: '/a.log', text: 'x'))
      ..open('b.log');

    expect(holder.opened, isNull);
    expect(holder.lines, isEmpty);
    expect((sent.single as OpenLog).name, 'b.log');

    holder.apply(
      const LogsUpdateOpened(
        name: 'b.log',
        path: '/b.log',
        text: 'first\nsecond\n',
      ),
    );

    expect(holder.opened?.name, 'b.log');
    expect(holder.lines, ['first', 'second']);
  });

  test('a failure goes out once and keeps the list', () async {
    const list = LogsUpdateSessions(dir: '/logs', sessions: []);
    holder.apply(list);
    final failures = <String>[];
    holder.failures.listen(failures.add);

    holder.apply(const LogsUpdateFailed(message: 'Could not list /logs'));

    expect(failures, ['Could not list /logs']);
    expect(holder.sessions, same(list));
  });
}
