import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/backup/backup_holder.dart';
import 'package:jitter/backup/backup_tiles.dart';
import 'package:jitter/src/bindings/bindings.dart';

void main() {
  late List<Object> sent;
  late BackupHolder holder;
  late List<(String, String)> saved;

  setUp(() {
    sent = [];
    saved = [];
    holder = BackupHolder(sendExport: sent.add, sendImport: sent.add);
  });

  Future<void> pumpTiles(
    WidgetTester tester, {
    PickBackup? pick,
    String? savedTo = '/home/me/jitter-backup.json',
  }) => tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: BackupTiles(
          backup: holder,
          save: (fileName, json) async {
            saved.add((fileName, json));
            return savedTo;
          },
          pick: pick ?? () async => null,
        ),
      ),
    ),
  );

  bool enabled(WidgetTester tester, String title) =>
      tester.widget<ListTile>(find.widgetWithText(ListTile, title)).enabled;

  testWidgets('export asks Rust and waits for its answer', (tester) async {
    await pumpTiles(tester);

    await tester.tap(find.text('Export data'));
    await tester.pump();

    expect(sent.single, isA<ExportBackup>());
    expect(enabled(tester, 'Export data'), isFalse);
    expect(enabled(tester, 'Import data'), isFalse);
  });

  testWidgets("Rust's backup is saved under its name, and where shows", (
    tester,
  ) async {
    await pumpTiles(tester);
    holder
      ..export()
      ..apply(
        const BackupUpdateExported(
          fileName: 'jitter-backup-2026-11-10.json',
          json: '{"format": "jitter-backup"}',
        ),
      );
    await tester.pump();

    expect(saved.single, (
      'jitter-backup-2026-11-10.json',
      '{"format": "jitter-backup"}',
    ));
    expect(enabled(tester, 'Export data'), isTrue);
    expect(
      find.text('Backup saved to /home/me/jitter-backup.json'),
      findsOneWidget,
    );
  });

  testWidgets('a cancelled save shows nothing', (tester) async {
    await pumpTiles(tester, savedTo: null);
    holder.apply(const BackupUpdateExported(fileName: 'b.json', json: '{}'));
    await tester.pump();

    expect(saved, hasLength(1));
    expect(find.byType(SnackBar), findsNothing);
  });

  testWidgets("import hands Rust the picked file's text", (tester) async {
    await pumpTiles(tester, pick: () async => '{"version": 1}');

    await tester.tap(find.text('Import data'));
    await tester.pump();

    expect(
      sent.single,
      isA<ImportBackup>().having((s) => s.json, 'json', '{"version": 1}'),
    );
  });

  testWidgets('a cancelled pick sends nothing', (tester) async {
    await pumpTiles(tester);

    await tester.tap(find.text('Import data'));
    await tester.pump();

    expect(sent, isEmpty);
    expect(enabled(tester, 'Import data'), isTrue);
  });

  testWidgets("Rust's import summary shows", (tester) async {
    await pumpTiles(tester);

    holder.apply(
      const BackupUpdateImported(
        summary: 'Imported 1 favorite, 2 visits and 0 resume points.',
      ),
    );
    await tester.pump();
    expect(
      find.text('Imported 1 favorite, 2 visits and 0 resume points.'),
      findsOneWidget,
    );
  });

  testWidgets("Rust's failure shows", (tester) async {
    await pumpTiles(tester);

    holder.apply(const BackupUpdateFailed(message: "Couldn't import: bad"));
    await tester.pump();

    expect(find.text("Couldn't import: bad"), findsOneWidget);
  });
}
