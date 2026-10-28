import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/playback_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';

void main() {
  test('starts idle', () {
    expect(PlaybackHolder().state, const PlaybackUpdateIdle());
  });

  test('each update replaces the state and notifies', () {
    final holder = PlaybackHolder();
    var notified = 0;
    holder.addListener(() => notified++);
    const ready = PlaybackUpdateReady(
      qualities: [
        QualityOption(name: '1080p60', label: '1080p60 (source)'),
        QualityOption(name: '720p60', label: '720p60'),
      ],
      selected: '1080p60',
      live: true,
      details: PlaybackDetails(chapters: [], game: 'Grand Theft Auto V'),
      sleep: PlaybackSleep(
        presets: [SleepPreset(minutes: 15, label: '15 minutes')],
      ),
      rewind: PlaybackRewindNone(),
    );

    holder
      ..apply(const PlaybackUpdateResolving())
      ..apply(ready);

    expect(holder.state, ready);
    expect(notified, 2);

    holder.apply(const PlaybackUpdateFailed(message: 'offline'));
    expect(holder.state, const PlaybackUpdateFailed(message: 'offline'));

    holder.apply(const PlaybackUpdateSlept());
    expect(holder.state, const PlaybackUpdateSlept());
  });
}
