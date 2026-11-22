import 'dart:async';
import 'dart:io';

import 'package:flutter/services.dart' show MethodChannel;
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
    expect(find.text('Download started'), findsNothing);

    holder.apply(const DownloadsUpdateStarted(videoId: '42'));
    await tester.pumpAndSettle();
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

  Future<void> chooseQuality(WidgetTester tester) async {
    await tester.tap(find.text('go'));
    await tester.pump();
    holder.apply(_choices);
    await tester.pumpAndSettle();
    await tester.tap(find.text('160p'));
    await tester.pumpAndSettle();
  }

  testWidgets('a start failure shows its message instead of started', (
    tester,
  ) async {
    await pumpFlow(tester);
    await chooseQuality(tester);

    holder.apply(
      const DownloadsUpdateFailed(videoId: '42', message: 'a.mp4 exists'),
    );
    await tester.pumpAndSettle();
    expect(find.text('a.mp4 exists'), findsOneWidget);
    expect(find.text('Download started'), findsNothing);
  });

  testWidgets("ignores another VOD's start", (tester) async {
    await pumpFlow(tester);
    await chooseQuality(tester);

    holder.apply(const DownloadsUpdateStarted(videoId: '7'));
    await tester.pumpAndSettle();
    expect(find.text('Download started'), findsNothing);
  });

  testWidgets('a start failure still shows after the page is gone', (
    tester,
  ) async {
    await pumpFlow(tester);
    await chooseQuality(tester);
    await tester.pumpWidget(
      const MaterialApp(home: Scaffold(body: Text('elsewhere'))),
    );

    holder.apply(
      const DownloadsUpdateFailed(videoId: '42', message: 'a.mp4 exists'),
    );
    await tester.pumpAndSettle();
    expect(find.text('a.mp4 exists'), findsOneWidget);
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

  group('androidDownloadDir', () {
    const channel = MethodChannel('jitter/storage');
    late Directory shared;

    setUp(() async {
      shared = await Directory.systemTemp.createTemp('shared_downloads');
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(
            channel,
            (call) async =>
                call.method == 'sharedDownloadsDir' ? shared.path : null,
          );
    });

    tearDown(() async {
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, null);
      await shared.delete(recursive: true);
    });

    test('creates Jitter in the shared Download folder', () async {
      final dir = await androidDownloadDir();

      expect(dir?.path, '${shared.path}/Jitter');
      expect(dir?.existsSync(), isTrue);
    });
  });
}
