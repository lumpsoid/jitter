import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/downloads/downloads_holder.dart';
import 'package:jitter/downloads/downloads_page.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:material_ui/material_ui.dart';

import 'fixtures.dart';

DownloadsUpdate _snapshot() => DownloadsUpdateSnapshot(
  downloads: [
    download(fileName: 'a.mp4', done: 3, total: 10, bytes: 5 * 1024 * 1024),
    download(
      fileName: 'b.mp4',
      key: 2,
      state: const DownloadStateDone(path: '/d/b.mp4'),
    ),
    download(
      fileName: 'c.mp4',
      key: 3,
      state: const DownloadStateFailed(message: '403'),
    ),
    download(fileName: 'd.mp4', key: 4, state: const DownloadStateCancelled()),
  ],
);

void main() {
  late DownloadsHolder holder;
  late List<CancelDownload> cancelled;
  late List<RemoveDownload> removed;

  setUp(() {
    cancelled = [];
    removed = [];
    holder = DownloadsHolder(
      sendCancel: cancelled.add,
      sendRemove: removed.add,
    );
  });

  Future<void> pumpPage(WidgetTester tester) =>
      tester.pumpWidget(MaterialApp(home: DownloadsPage(downloads: holder)));

  testWidgets('says so when there are no downloads', (tester) async {
    await pumpPage(tester);

    expect(
      find.text('No downloads yet. Long-press a VOD to download it.'),
      findsOneWidget,
    );
  });

  testWidgets('shows each download by its state', (tester) async {
    await pumpPage(tester);
    holder.apply(_snapshot());
    await tester.pump();

    expect(find.text('3 / 10 segments · 5.0 MiB'), findsOneWidget);
    expect(find.text('Saved to /d/b.mp4'), findsOneWidget);
    expect(find.text('Failed: 403'), findsOneWidget);
    expect(find.text('Cancelled'), findsOneWidget);

    expect(find.byType(LinearProgressIndicator), findsOneWidget);
    final progress = tester.widget<LinearProgressIndicator>(
      find.byType(LinearProgressIndicator),
    );
    expect(progress.value, closeTo(0.3, 1e-9));

    expect(find.byTooltip('Cancel'), findsOneWidget);
    expect(find.byTooltip('Remove'), findsNWidgets(3));
  });

  testWidgets('cancel and remove send the item key', (tester) async {
    await pumpPage(tester);
    holder.apply(_snapshot());
    await tester.pump();

    await tester.tap(find.byTooltip('Cancel'));
    expect(cancelled, [CancelDownload(key: Uint64.parse('1'))]);

    await tester.tap(
      find.descendant(
        of: find.byKey(ValueKey(Uint64.parse('2'))),
        matching: find.byTooltip('Remove'),
      ),
    );
    expect(removed, [RemoveDownload(key: Uint64.parse('2'))]);
  });

  testWidgets('a failure shows a snackbar', (tester) async {
    await pumpPage(tester);

    holder.apply(const DownloadsUpdateFailed(videoId: '42', message: 'boom'));
    await tester.pump();

    expect(find.text('boom'), findsOneWidget);
  });
}
