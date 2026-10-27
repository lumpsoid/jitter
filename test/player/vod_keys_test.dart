import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/vod_keys.dart';

/// Pumps the keys on a focused box; each callback they call is added to
/// the list returned.
Future<List<String>> pumpKeys(WidgetTester tester) async {
  final calls = <String>[];
  await tester.pumpWidget(
    CallbackShortcuts(
      bindings: vodKeys(
        onTogglePlay: () => calls.add('toggle'),
        onPlay: () => calls.add('play'),
        onPause: () => calls.add('pause'),
        onSeekBy: (by) => calls.add('seek ${by.inSeconds}'),
        onStepFrame: (forward) =>
            calls.add(forward ? 'frame on' : 'frame back'),
        onNudgeVolume: (by) => calls.add('volume $by'),
        onToggleFullscreen: () => calls.add('fullscreen'),
        onExitFullscreen: () => calls.add('exit'),
      ),
      child: const Focus(autofocus: true, child: SizedBox()),
    ),
  );
  return calls;
}

Future<void> shifted(WidgetTester tester, LogicalKeyboardKey key) async {
  await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
  await tester.sendKeyEvent(key);
  await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
}

void main() {
  testWidgets('the arrows go 10 s, with Shift 1 s', (tester) async {
    final calls = await pumpKeys(tester);

    await tester.sendKeyEvent(LogicalKeyboardKey.arrowLeft);
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    await shifted(tester, LogicalKeyboardKey.arrowLeft);
    await shifted(tester, LogicalKeyboardKey.arrowRight);

    expect(calls, ['seek -10', 'seek 10', 'seek -1', 'seek 1']);
  });

  testWidgets('comma and period step a frame back and on', (tester) async {
    final calls = await pumpKeys(tester);

    await tester.sendKeyEvent(LogicalKeyboardKey.comma, character: ',');
    await tester.sendKeyEvent(LogicalKeyboardKey.period, character: '.');

    expect(calls, ['frame back', 'frame on']);
  });

  testWidgets("media_kit's other keys stay as they were", (tester) async {
    final calls = await pumpKeys(tester);

    for (final key in [
      LogicalKeyboardKey.space,
      LogicalKeyboardKey.mediaPlay,
      LogicalKeyboardKey.mediaPause,
      LogicalKeyboardKey.keyJ,
      LogicalKeyboardKey.keyI,
      LogicalKeyboardKey.arrowUp,
      LogicalKeyboardKey.arrowDown,
      LogicalKeyboardKey.keyF,
      LogicalKeyboardKey.escape,
    ]) {
      await tester.sendKeyEvent(key);
    }

    expect(calls, [
      'toggle',
      'play',
      'pause',
      'seek -10',
      'seek 10',
      'volume 5.0',
      'volume -5.0',
      'fullscreen',
      'exit',
    ]);
  });
}
