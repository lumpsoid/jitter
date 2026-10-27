import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/faded_volume.dart';

void main() {
  test('a fade plays below the user volume and ends back at it', () {
    final volume = FadedVolume();

    expect(volume.fade(0.5, current: 80), 40);
    // The player now reads 40: the user's 80 is kept.
    expect(volume.fade(0.25, current: 40), 20);
    expect(volume.end(), 80);
    expect(volume.end(), isNull);
  });

  test('a fade of 1 ends the fade; with none playing, nothing changes', () {
    final volume = FadedVolume();

    expect(volume.fade(1, current: 70), isNull);
    volume.fade(0.5, current: 70);
    expect(volume.fade(1, current: 35), 70);
  });

  test('without a fade, mute toggles as before', () {
    final volume = FadedVolume();

    expect(volume.toggleMute(current: 60), 0);
    expect(volume.toggleMute(current: 0), 60);
  });

  test('mute mid-fade mutes the user volume, and the end keeps it muted', () {
    final volume = FadedVolume()..fade(0.5, current: 80);

    expect(volume.toggleMute(current: 40), 0);
    expect(volume.fade(0.25, current: 0), 0);
    expect(volume.end(), 0);
    expect(volume.toggleMute(current: 0), 80);
  });

  test('unmute mid-fade comes back under the fade', () {
    final volume = FadedVolume()
      ..toggleMute(current: 60)
      ..fade(0.5, current: 0);

    expect(volume.toggleMute(current: 0), 30);
    expect(volume.end(), 60);
  });

  test('a nudge moves the volume within 0–100', () {
    final volume = FadedVolume();

    expect(volume.nudge(5, current: 60), 65);
    expect(volume.nudge(5, current: 98), 100);
    expect(volume.nudge(-5, current: 3), 0);
  });

  test('a nudge mid-fade moves the user volume under the fade', () {
    final volume = FadedVolume()..fade(0.5, current: 80);

    expect(volume.nudge(10, current: 40), 45);
    expect(volume.end(), 90);
  });
}
