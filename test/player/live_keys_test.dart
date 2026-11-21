import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/live_keys.dart';

/// The keys with each callback recording its call in the list returned,
/// pumped on a focused box.
Future<(Map<ShortcutActivator, VoidCallback>, List<String>)> pumpKeys(
  WidgetTester tester,
) async {
  final calls = <String>[];
  final keys = liveKeys(
    onTogglePlay: () => calls.add('toggle'),
    onPlay: () => calls.add('play'),
    onPause: () => calls.add('pause'),
    onNudgeVolume: (by) => calls.add('volume $by'),
    onToggleFullscreen: () => calls.add('fullscreen'),
    onExitFullscreen: () => calls.add('exit'),
  );
  await tester.pumpWidget(
    CallbackShortcuts(
      bindings: keys,
      child: const Focus(autofocus: true, child: SizedBox()),
    ),
  );
  return (keys, calls);
}

void main() {
  testWidgets('the map has exactly the eight keys of media_kit without seeks', (
    tester,
  ) async {
    final (keys, _) = await pumpKeys(tester);

    expect(keys.keys.toSet(), {
      const SingleActivator(LogicalKeyboardKey.mediaPlay),
      const SingleActivator(LogicalKeyboardKey.mediaPause),
      const SingleActivator(LogicalKeyboardKey.mediaPlayPause),
      const SingleActivator(LogicalKeyboardKey.space),
      const SingleActivator(LogicalKeyboardKey.arrowUp),
      const SingleActivator(LogicalKeyboardKey.arrowDown),
      const SingleActivator(LogicalKeyboardKey.keyF),
      const SingleActivator(LogicalKeyboardKey.escape),
    });
  });

  testWidgets('no seek key is bound: a live stream cannot seek', (
    tester,
  ) async {
    final (keys, calls) = await pumpKeys(tester);

    // The positive control: a bound key is found in the same map.
    expect(
      keys.containsKey(const SingleActivator(LogicalKeyboardKey.space)),
      isTrue,
    );
    expect(
      keys.containsKey(const SingleActivator(LogicalKeyboardKey.arrowLeft)),
      isFalse,
    );
    expect(
      keys.containsKey(const SingleActivator(LogicalKeyboardKey.arrowRight)),
      isFalse,
    );
    expect(
      keys.containsKey(const SingleActivator(LogicalKeyboardKey.keyJ)),
      isFalse,
    );
    expect(
      keys.containsKey(const SingleActivator(LogicalKeyboardKey.keyI)),
      isFalse,
    );

    await tester.sendKeyEvent(LogicalKeyboardKey.arrowLeft);
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyJ);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyI);

    expect(calls, isEmpty);
  });

  testWidgets('space toggles, and the arrows nudge the volume by 5', (
    tester,
  ) async {
    final (_, calls) = await pumpKeys(tester);

    await tester.sendKeyEvent(LogicalKeyboardKey.space);
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowUp);
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowDown);

    expect(calls, ['toggle', 'volume 5.0', 'volume -5.0']);
  });
}
