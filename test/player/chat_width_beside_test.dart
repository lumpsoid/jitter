import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/now_playing.dart';
import 'package:jitter/player/player_layout.dart';

void main() {
  test('a 16:9 window gives the chat its share of the width', () {
    expect(chatWidthBeside(const Size(1920, 1000), nowPlaying: true), 340);
    expect(chatWidthBeside(const Size(1200, 744), nowPlaying: true), 300);
    expect(chatWidthBeside(const Size(900, 744), nowPlaying: true), 240);
  });

  test('a wider window gives the chat the room the video leaves', () {
    // 948 high, the video is 1685.3 wide.
    expect(
      chatWidthBeside(const Size(2200, 1000), nowPlaying: true),
      closeTo(2200 - 1 - 948 * 16 / 9, 0.01),
    );
  });

  test('without the strip the video is taller and leaves less room', () {
    expect(
      chatWidthBeside(const Size(2200, 1000), nowPlaying: false),
      closeTo(2200 - 1 - 1000 * 16 / 9, 0.01),
    );
  });

  test('an ultrawide window caps the chat, the video between bars', () {
    expect(chatWidthBeside(const Size(2560, 1000), nowPlaying: true), 640);
    expect(chatWidthBeside(const Size(5120, 1300), nowPlaying: true), 640);
  });

  test('an unbounded height falls back to the share', () {
    expect(
      chatWidthBeside(const Size(1200, double.infinity), nowPlaying: true),
      300,
    );
  });

  testWidgets('the strip is as high as the layout reckons', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Align(
          alignment: Alignment.topCenter,
          child: NowPlaying(
            title: 'Title',
            game: 'Game',
            onGoToChapter: (_) {},
          ),
        ),
      ),
    );
    // The chat's width is what a 16:9 video under the real strip leaves.
    final strip = tester.getSize(find.byType(NowPlaying)).height;
    expect(
      chatWidthBeside(const Size(2200, 1000), nowPlaying: true),
      closeTo(2200 - 1 - (1000 - strip) * 16 / 9, 0.01),
    );
  });
}
