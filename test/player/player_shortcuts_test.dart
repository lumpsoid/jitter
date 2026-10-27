import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/player_shortcuts.dart';

/// Pumps the shortcuts around a button, a text field and a stand-in for
/// media_kit's video, which handles Space itself while it has the focus.
/// Returns what was pressed, by name, in order.
Future<List<String>> pumpShortcuts(
  WidgetTester tester, {
  FocusNode? video,
}) async {
  final pressed = <String>[];
  await tester.pumpWidget(
    MaterialApp(
      home: PlayerShortcuts(
        onToggleChat: () => pressed.add('chat'),
        onToggleMute: () => pressed.add('mute'),
        onToggleFullscreen: () => pressed.add('fullscreen'),
        onTogglePlay: () => pressed.add('play'),
        onToggleTheatre: () => pressed.add('theatre'),
        onZoomIn: () => pressed.add('zoom in'),
        onZoomOut: () => pressed.add('zoom out'),
        onResetZoom: () => pressed.add('reset zoom'),
        child: Scaffold(
          body: Column(
            children: [
              TextButton(
                onPressed: () => pressed.add('button'),
                child: const Text('Button'),
              ),
              const TextField(),
              CallbackShortcuts(
                bindings: {
                  const SingleActivator(LogicalKeyboardKey.space): () =>
                      pressed.add('video play'),
                },
                child: Focus(focusNode: video, child: const Text('Video')),
              ),
            ],
          ),
        ),
      ),
    ),
  );
  return pressed;
}

void main() {
  testWidgets('each key toggles its part as soon as the page opens', (
    tester,
  ) async {
    final pressed = await pumpShortcuts(tester);

    await tester.sendKeyEvent(LogicalKeyboardKey.keyC);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyM);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyF);
    await tester.sendKeyEvent(LogicalKeyboardKey.space);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyT);

    expect(pressed, ['chat', 'mute', 'fullscreen', 'play', 'theatre']);
  });

  testWidgets('+ and = zoom in, - zooms out and 0 resets the zoom', (
    tester,
  ) async {
    final pressed = await pumpShortcuts(tester);

    await tester.sendKeyEvent(LogicalKeyboardKey.numpadAdd, character: '+');
    await tester.sendKeyEvent(LogicalKeyboardKey.equal, character: '=');
    await tester.sendKeyEvent(LogicalKeyboardKey.minus, character: '-');
    await tester.sendKeyEvent(LogicalKeyboardKey.digit0, character: '0');

    expect(pressed, ['zoom in', 'zoom in', 'zoom out', 'reset zoom']);
  });

  testWidgets('Space plays or pauses while a button has the focus', (
    tester,
  ) async {
    final pressed = await pumpShortcuts(tester);
    Focus.of(tester.element(find.text('Button'))).requestFocus();
    await tester.pump();

    await tester.sendKeyEvent(LogicalKeyboardKey.space);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyC);

    expect(pressed, ['play', 'chat']);
  });

  testWidgets('keys type into a focused text field', (tester) async {
    final pressed = await pumpShortcuts(tester);
    await tester.tap(find.byType(TextField));
    await tester.pump();

    await tester.sendKeyEvent(LogicalKeyboardKey.keyC);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyT);
    await tester.sendKeyEvent(LogicalKeyboardKey.space);

    expect(pressed, isEmpty);
  });

  testWidgets('a key the video handles fires once, the rest reach the page', (
    tester,
  ) async {
    final video = FocusNode();
    addTearDown(video.dispose);
    final pressed = await pumpShortcuts(tester, video: video);
    video.requestFocus();
    await tester.pump();

    await tester.sendKeyEvent(LogicalKeyboardKey.space);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyM);

    expect(pressed, ['video play', 'mute']);
  });
}
