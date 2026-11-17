import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/controls_fade.dart';
import 'package:jitter/player/live.dart';
import 'package:jitter/player/live_badge.dart';
import 'package:jitter/player/live_mode_slot.dart';
import 'package:jitter/player/player_buttons.dart';
import 'package:jitter/player/position_readout.dart';
import 'package:jitter/player/rewind_messages.dart';
import 'package:jitter/player/seek_bar.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:material_ui/material_ui.dart';

/// The widgets the desktop bar shows, under a half fade: each one's colours
/// are read back at half their alpha. Each setup is copied from the widget's
/// own test file.
const _half = AlwaysStoppedAnimation<double>(0.5);

/// A canvas that keeps the colour of each rectangle as it is drawn, so a
/// painter's colours can be read without its private class.
class _Drawn implements Canvas {
  final colors = <Color>[];

  @override
  void drawRect(Rect rect, Paint paint) => colors.add(paint.color);

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

void main() {
  testWidgets('the readout fades by its text colour', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ControlsFade(
            notifier: _half,
            child: PositionReadout(
              positions: Live(
                now: () => const Duration(hours: 1),
                changes: Stream.empty,
              ),
              durations: Live(
                now: () => const Duration(hours: 6),
                changes: Stream.empty,
              ),
              answers: () => const Stream.empty(),
              onGoTo: (_) {},
              fontSize: 14,
            ),
          ),
        ),
      ),
    );

    final text = tester.widget<Text>(find.text('1:00:00 / 6:00:00'));
    expect(text.style!.color!.a, closeTo(0.5, 0.01));
  });

  for (final behind in [false, true]) {
    testWidgets(
      'a LIVE badge ${behind ? 'behind' : 'at'} the live edge fades its '
      'background and text',
      (tester) async {
        await tester.pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: ControlsFade(
                notifier: _half,
                child: LiveBadge(
                  style: const TextStyle(fontSize: 12),
                  behind: behind,
                ),
              ),
            ),
          ),
        );

        final colors = Theme.of(tester.element(find.byType(LiveBadge)))
            .colorScheme;
        final box = tester.widget<DecoratedBox>(
          find.descendant(
            of: find.byType(LiveBadge),
            matching: find.byType(DecoratedBox),
          ),
        );
        final background = (box.decoration as BoxDecoration).color!;
        final text = tester.widget<Text>(find.text('LIVE')).style!.color!;
        expect(
          background.a,
          closeTo((behind ? colors.outline : colors.error).a * 0.5, 0.01),
        );
        expect(
          text.a,
          closeTo((behind ? colors.surface : colors.onError).a * 0.5, 0.01),
        );
      },
    );
  }

  testWidgets('the seek bar fades its track and its played colour', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Center(
            child: ControlsFade(
              notifier: _half,
              child: SizedBox(
                width: 400,
                child: SeekBar(
                  positions: Live(
                    now: () => const Duration(hours: 1),
                    changes: Stream.empty,
                  ),
                  durations: Live(
                    now: () => const Duration(hours: 6),
                    changes: Stream.empty,
                  ),
                  buffers: Live(
                    now: () => Duration.zero,
                    changes: Stream.empty,
                  ),
                  onSeek: (_) {},
                ),
              ),
            ),
          ),
        ),
      ),
    );

    final painter = tester
        .widget<CustomPaint>(
          find.descendant(
            of: find.byType(SeekBar),
            matching: find.byType(CustomPaint),
          ),
        )
        .painter!;
    final drawn = _Drawn();
    painter.paint(drawn, const Size(400, 36));

    // The whole track, the buffered part, then the played part.
    expect(drawn.colors, hasLength(3));
    expect(drawn.colors.first.a, closeTo(0x3D / 255 * 0.5, 0.01));
    expect(drawn.colors.last.a, closeTo(0.5, 0.01));
  });

  testWidgets('the rewind button fades by its foreground colour', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Material(
          color: Colors.black,
          child: SizedBox(
            width: 800,
            height: 450,
            child: RewindMessages(
              answers: () => const Stream.empty(),
              child: Align(
                alignment: Alignment.bottomLeft,
                child: ControlsFade(
                  notifier: _half,
                  child: LiveModeSlot(
                    rewind: const PlaybackRewindOffered(),
                    onRewind: () {},
                    onBackToLive: () {},
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pump();

    final button = tester.widget<TextButton>(
      find.ancestor(
        of: find.text('Rewind'),
        matching: find.bySubtype<TextButton>(),
      ),
    );
    final foreground = button.style!.foregroundColor!.resolve(const {});
    expect(foreground!.a, closeTo(0.5, 0.01));
  });

  testWidgets('a button icon follows the fade with no lag of its own', (
    tester,
  ) async {
    final fade = AnimationController(vsync: const TestVSync(), value: 1);
    addTearDown(fade.dispose);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Center(
            child: ControlsFade(
              notifier: fade,
              child: FullscreenButton(fullscreen: false, onPressed: () {}),
            ),
          ),
        ),
      ),
    );

    fade.value = 0.5;
    await tester.pump();

    final icon = tester.widget<RichText>(
      find.descendant(
        of: find.byType(FullscreenButton),
        matching: find.byType(RichText),
      ),
    );
    expect(icon.text.style!.color!.a, closeTo(0.5, 0.01));
  });

  testWidgets('the rewind button label and icon follow the fade with no lag', (
    tester,
  ) async {
    final fade = AnimationController(vsync: const TestVSync(), value: 1);
    addTearDown(fade.dispose);
    await tester.pumpWidget(
      MaterialApp(
        home: Material(
          color: Colors.black,
          child: SizedBox(
            width: 800,
            height: 450,
            child: RewindMessages(
              answers: () => const Stream.empty(),
              child: Align(
                alignment: Alignment.bottomLeft,
                child: ControlsFade(
                  notifier: fade,
                  child: LiveModeSlot(
                    rewind: const PlaybackRewindOffered(),
                    onRewind: () {},
                    onBackToLive: () {},
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );

    fade.value = 0.5;
    await tester.pump();

    final label = tester.widget<RichText>(
      find.descendant(of: find.text('Rewind'), matching: find.byType(RichText)),
    );
    expect(label.text.style!.color!.a, closeTo(0.5, 0.01));

    final icon = tester.widget<RichText>(
      find.descendant(
        of: find.byIcon(Icons.history),
        matching: find.byType(RichText),
      ),
    );
    expect(icon.text.style!.color!.a, closeTo(0.5, 0.01));
  });
}
