import 'package:audio_service/audio_service.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/media_session.dart';
import 'package:jitter/src/bindings/bindings.dart';

PlaybackUpdateReady ready(PlaybackDetails details, {bool live = true}) =>
    PlaybackUpdateReady(
      qualities: const [QualityOption(name: '1080p60', label: '1080p60')],
      selected: '1080p60',
      live: live,
      details: details,
      sleep: const PlaybackSleep(presets: []),
    );

void main() {
  test('a playing page shows its channel, game and avatar', () {
    final item = mediaItemOf(
      ready(
        const PlaybackDetails(
          channel: PlaybackChannel(
            login: 'xqc',
            displayName: 'xQc',
            avatarUrl: 'https://example.test/xqc.jpeg',
          ),
          game: 'Just Chatting',
        ),
      ),
    );

    expect(item?.id, 'xqc');
    expect(item?.title, 'xQc');
    expect(item?.artist, 'Just Chatting');
    expect(item?.artUri, Uri.parse('https://example.test/xqc.jpeg'));
    expect(item?.isLive, isTrue);
  });

  test('a page without details still shows the app', () {
    final item = mediaItemOf(ready(const PlaybackDetails(), live: false));

    expect(item?.title, 'Jitter');
    expect(item?.artist, isNull);
    expect(item?.artUri, isNull);
    expect(item?.isLive, isFalse);
  });

  test('nothing is shown outside a playing page', () {
    for (final state in const [
      PlaybackUpdateIdle(),
      PlaybackUpdateResolving(),
      PlaybackUpdateSlept(),
      PlaybackUpdateFailed(message: 'offline'),
    ]) {
      expect(mediaItemOf(state), isNull, reason: '$state');
      final session = sessionStateOf(state, playing: true);
      expect(session.processingState, AudioProcessingState.idle);
      expect(session.playing, isFalse);
      expect(session.controls, isEmpty);
    }
  });

  test('a playing page offers pause, a paused one play', () {
    final page = ready(const PlaybackDetails());

    final playing = sessionStateOf(page, playing: true);
    expect(playing.processingState, AudioProcessingState.ready);
    expect(playing.playing, isTrue);
    expect(playing.controls, [MediaControl.pause]);

    final paused = sessionStateOf(page, playing: false);
    expect(paused.playing, isFalse);
    expect(paused.controls, [MediaControl.play]);
  });
}
