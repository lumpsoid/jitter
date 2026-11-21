import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/downloads/downloads_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';

import 'fixtures.dart';

void main() {
  late DownloadsHolder holder;
  late List<ChooseDownload> chosen;
  late List<StartDownload> started;
  late List<CancelDownload> cancelled;
  late List<RemoveDownload> removed;
  late int notifications;

  setUp(() {
    chosen = [];
    started = [];
    cancelled = [];
    removed = [];
    holder = DownloadsHolder(
      sendChoose: chosen.add,
      sendStart: started.add,
      sendCancel: cancelled.add,
      sendRemove: removed.add,
    );
    notifications = 0;
    holder.addListener(() => notifications++);
  });

  test('has no downloads until a snapshot', () {
    expect(holder.downloads, isEmpty);
    expect(notifications, 0);
  });

  test('a snapshot replaces the downloads', () {
    final first = [
      download(fileName: 'a.mp4'),
      download(
        fileName: 'b.mp4',
        key: 2,
        state: const DownloadStateDone(path: '/d/a.mp4'),
      ),
    ];
    holder.apply(DownloadsUpdateSnapshot(downloads: first));
    expect(holder.downloads, equals(first));
    expect(holder.downloads, hasLength(2));
    expect(holder.downloads.first.fileName, 'a.mp4');

    final second = [download(fileName: 'c.mp4', key: 3)];
    holder.apply(DownloadsUpdateSnapshot(downloads: second));
    expect(holder.downloads, equals(second));
    expect(holder.downloads, hasLength(1));
    expect(notifications, 2);
  });

  test("choose answers with its VOD's choices and leaves the downloads "
      'alone', () async {
    final before = [download(fileName: 'a.mp4')];
    holder.apply(DownloadsUpdateSnapshot(downloads: before));
    final answer = holder.choose('42');

    const mine = DownloadsUpdateChoices(
      videoId: '42',
      qualities: [QualityOption(name: '720p60', label: '720p60')],
    );
    holder
      ..apply(const DownloadsUpdateChoices(videoId: '7', qualities: []))
      ..apply(mine);
    expect(await answer, mine);
    expect(holder.downloads, equals(before));
    expect(notifications, 1);
  });

  test("choose answers with its VOD's failure, skipping another's", () async {
    final answer = holder.choose('42');

    const mine = DownloadsUpdateFailed(videoId: '42', message: 'gone');
    holder
      ..apply(const DownloadsUpdateFailed(videoId: '7', message: 'other'))
      ..apply(mine);
    expect(await answer, mine);
  });

  test('a failure goes out once and leaves the downloads alone', () {
    final before = [download(fileName: 'a.mp4')];
    holder.apply(DownloadsUpdateSnapshot(downloads: before));
    final received = <String>[];
    holder.failures.listen(received.add);

    holder.apply(const DownloadsUpdateFailed(videoId: '42', message: 'x'));
    expect(received, ['x']);
    expect(holder.downloads, equals(before));
    expect(notifications, 1);
  });

  test('each ask sends its signal', () {
    final key = Uint64.parse('7');
    unawaited(holder.choose('42'));
    holder
      ..start(videoId: '42', quality: '720p60', dir: '/d')
      ..cancel(key)
      ..remove(key);
    expect(chosen, const [ChooseDownload(videoId: '42')]);
    expect(started, const [
      StartDownload(videoId: '42', quality: '720p60', dir: '/d'),
    ]);
    expect(cancelled, [CancelDownload(key: key)]);
    expect(removed, [RemoveDownload(key: key)]);
    expect(notifications, 0);
  });
}
