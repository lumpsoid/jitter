import 'dart:async';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/downloads/download_flow.dart';
import 'package:jitter/downloads/downloads_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:material_ui/material_ui.dart';

const _choices = DownloadsUpdateChoices(
  videoId: '42',
  qualities: [
    QualityOption(name: '720p60', label: '720p60'),
    QualityOption(name: '160p30', label: '160p'),
  ],
);

Future<String> _dir() async => '/d';

Future<String> _noDir() async => throw const FileSystemException('nope');

void main() {
  late DownloadsHolder holder;
  late List<ChooseDownload> chosen;
  late List<StartDownload> started;
  late int viewed;

  setUp(() {
    chosen = [];
    started = [];
    viewed = 0;
    holder = DownloadsHolder(sendChoose: chosen.add, sendStart: started.add);
  });

  Future<void> pumpFlow(
    WidgetTester tester, {
    Future<String> Function() dir = _dir,
  }) => tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: Builder(
          builder: (context) => TextButton(
            onPressed: () => unawaited(
              downloadVod(
                context,
                downloads: holder,
                videoId: '42',
                downloadDir: dir,
                onView: () => viewed++,
              ),
            ),
            child: const Text('go'),
          ),
        ),
      ),
    ),
  );

  testWidgets('asks for the qualities, starts the chosen one and offers View', (
    tester,
  ) async {
    await pumpFlow(tester);
    await tester.tap(find.text('go'));
    await tester.pump();
    expect(chosen, [const ChooseDownload(videoId: '42')]);

    holder.apply(_choices);
    await tester.pumpAndSettle();
    expect(find.text('Download in quality'), findsOneWidget);
    await tester.tap(find.text('160p'));
    await tester.pumpAndSettle();
    expect(started, [
      const StartDownload(videoId: '42', quality: '160p30', dir: '/d'),
    ]);
    expect(find.text('Download started'), findsOneWidget);
    await tester.tap(find.text('View'));
    await tester.pumpAndSettle();
    expect(viewed, 1);
  });

  testWidgets('ignores choices for another VOD', (tester) async {
    await pumpFlow(tester);
    await tester.tap(find.text('go'));
    await tester.pump();

    holder.apply(
      const DownloadsUpdateChoices(
        videoId: '7',
        qualities: [QualityOption(name: 'x', label: 'x')],
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Download in quality'), findsNothing);

    holder.apply(_choices);
    await tester.pumpAndSettle();
    expect(find.text('Download in quality'), findsOneWidget);
  });

  testWidgets('a failure shows its message and starts nothing', (tester) async {
    await pumpFlow(tester);
    await tester.tap(find.text('go'));
    await tester.pump();

    holder.apply(
      const DownloadsUpdateFailed(videoId: '42', message: 'no such vod'),
    );
    await tester.pumpAndSettle();
    expect(find.text('no such vod'), findsOneWidget);
    expect(started, isEmpty);
  });

  testWidgets('ignores a failure for another VOD', (tester) async {
    await pumpFlow(tester);
    await tester.tap(find.text('go'));
    await tester.pump();

    holder.apply(
      const DownloadsUpdateFailed(videoId: '7', message: 'not this one'),
    );
    await tester.pumpAndSettle();
    expect(find.text('not this one'), findsNothing);

    holder.apply(_choices);
    await tester.pumpAndSettle();
    expect(find.text('Download in quality'), findsOneWidget);
  });

  testWidgets('dismissing the sheet starts nothing', (tester) async {
    await pumpFlow(tester);
    await tester.tap(find.text('go'));
    await tester.pump();
    holder.apply(_choices);
    await tester.pumpAndSettle();
    expect(find.text('Download in quality'), findsOneWidget);

    await tester.tapAt(const Offset(10, 10));
    await tester.pumpAndSettle();
    expect(find.text('Download in quality'), findsNothing);
    expect(started, isEmpty);
  });

  testWidgets('no folder says so and starts nothing', (tester) async {
    await pumpFlow(tester, dir: _noDir);
    await tester.tap(find.text('go'));
    await tester.pump();
    holder.apply(_choices);
    await tester.pumpAndSettle();
    await tester.tap(find.text('160p'));
    await tester.pumpAndSettle();

    expect(find.textContaining('No folder to save into'), findsOneWidget);
    expect(started, isEmpty);
  });
}
