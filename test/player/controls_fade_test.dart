import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/controls_fade.dart';
import 'package:jitter/player/live.dart';
import 'package:jitter/player/player_buttons.dart';
import 'package:material_ui/material_ui.dart';

void main() {
  testWidgets('with no ControlsFade above, a colour is not faded', (
    tester,
  ) async {
    late Color faded;
    await tester.pumpWidget(
      Builder(
        builder: (context) {
          faded = fadedColor(context, const Color(0xFFFFFFFF));
          return const SizedBox();
        },
      ),
    );

    expect(faded, const Color(0xFFFFFFFF));
  });

  testWidgets('a ControlsFade scales a colour alpha by its value', (
    tester,
  ) async {
    late Color opaque;
    late Color translucent;
    await tester.pumpWidget(
      ControlsFade(
        notifier: const AlwaysStoppedAnimation<double>(0.5),
        child: Builder(
          builder: (context) {
            opaque = fadedColor(context, const Color(0xFFFFFFFF));
            translucent = fadedColor(context, const Color(0x80FFFFFF));
            return const SizedBox();
          },
        ),
      ),
    );

    expect(opaque.a, closeTo(0.5, 0.01));
    expect(translucent.a, closeTo(0.25, 0.01));
  });

  testWidgets('a play/pause button under a half fade draws half alpha', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Center(
            child: ControlsFade(
              notifier: const AlwaysStoppedAnimation<double>(0.5),
              child: PlayPauseButton(
                playing: Live(now: () => false, changes: Stream<bool>.empty),
                onPressed: () {},
              ),
            ),
          ),
        ),
      ),
    );

    final button = tester.widget<IconButton>(find.byType(IconButton));
    expect(button.color!.a, closeTo(0.5, 0.01));
  });
}
