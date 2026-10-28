import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/live_mode_slot.dart';
import 'package:jitter/player/rewind_messages.dart';
import 'package:jitter/src/bindings/bindings.dart';

/// What the slot asked of Rust.
class Sent {
  int rewinds = 0;
  int returns = 0;
}

/// Pumps the slot for [rewind] over a video-sized area, within the
/// messages that show Rust's [answers].
Future<Sent> pumpSlot(
  WidgetTester tester,
  PlaybackRewind rewind, {
  Stream<RewindAnswer>? answers,
}) async {
  final sent = Sent();
  await tester.pumpWidget(
    MaterialApp(
      home: Material(
        color: Colors.black,
        child: SizedBox(
          width: 800,
          height: 450,
          child: RewindMessages(
            answers: () => answers ?? const Stream.empty(),
            child: Align(
              alignment: Alignment.bottomLeft,
              child: LiveModeSlot(
                rewind: rewind,
                onRewind: () => sent.rewinds++,
                onBackToLive: () => sent.returns++,
              ),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pump();
  return sent;
}

/// Sends Rust's [message] and lets it reach the screen: the stream
/// delivers it during one frame, which the next one shows.
Future<void> answer(
  WidgetTester tester,
  StreamController<RewindAnswer> answers,
  RewindAnswer message,
) async {
  answers.add(message);
  await tester.pump();
  await tester.pump();
}

const _reason = "Twitch doesn't show a recording of this broadcast.";

void main() {
  testWidgets('at the live edge it offers a rewind; a tap asks for one', (
    tester,
  ) async {
    final sent = await pumpSlot(tester, const PlaybackRewindOffered());
    expect(find.text('LIVE'), findsOneWidget);
    expect(find.byIcon(Icons.history), findsOneWidget);

    await tester.tap(find.text('Rewind stream'));
    await tester.pump();
    expect(sent.rewinds, 1);
    expect(sent.returns, 0);
  });

  testWidgets('after a failed try it offers a retry, and says why', (
    tester,
  ) async {
    final sent = await pumpSlot(
      tester,
      const PlaybackRewindOffered(lastFailure: 'Timed out.'),
    );
    expect(find.byTooltip('Timed out.'), findsOneWidget);

    await tester.tap(find.text('Rewind stream'));
    await tester.pump();
    expect(sent.rewinds, 1);
  });

  testWidgets('a plain offer has no tooltip', (tester) async {
    await pumpSlot(tester, const PlaybackRewindOffered());
    expect(find.byType(Tooltip), findsNothing);
  });

  testWidgets('while it loads a spinner shows and a tap does nothing', (
    tester,
  ) async {
    final sent = await pumpSlot(tester, const PlaybackRewindLoading());
    expect(find.text('LIVE'), findsOneWidget);
    expect(find.text('Rewind stream'), findsOneWidget);
    expect(find.byType(CircularProgressIndicator), findsOneWidget);
    expect(find.byIcon(Icons.history), findsNothing);

    await tester.tap(find.text('Rewind stream'));
    await tester.pump();
    expect(sent.rewinds, 0);
    expect(sent.returns, 0);
  });

  testWidgets('when unavailable a tap shows why and asks for nothing', (
    tester,
  ) async {
    final sent = await pumpSlot(
      tester,
      const PlaybackRewindUnavailable(reason: _reason),
    );
    expect(find.text('LIVE'), findsOneWidget);
    expect(find.byTooltip(_reason), findsOneWidget);
    expect(find.text(_reason), findsNothing);

    await tester.tap(find.text('Rewind stream'));
    await tester.pump();
    expect(find.text(_reason), findsOneWidget);
    expect(sent.rewinds, 0);
    expect(sent.returns, 0);

    await tester.pump(const Duration(seconds: 4));
    expect(find.text(_reason), findsNothing);
  });

  testWidgets('rewound it offers the way back to live; a tap asks for it', (
    tester,
  ) async {
    final sent = await pumpSlot(tester, const PlaybackRewindRewound());
    expect(find.text('LIVE'), findsOneWidget);
    expect(find.text('Rewind stream'), findsNothing);
    expect(find.byTooltip('Back to live'), findsOneWidget);
    expect(find.byIcon(Icons.last_page), findsOneWidget);

    await tester.tap(find.text('LIVE'));
    await tester.pump();
    expect(sent.returns, 1);
    expect(sent.rewinds, 0);
  });

  testWidgets('while it returns a spinner shows and a tap does nothing', (
    tester,
  ) async {
    final sent = await pumpSlot(tester, const PlaybackRewindReturning());
    expect(find.byTooltip('Back to live'), findsOneWidget);
    expect(find.byType(CircularProgressIndicator), findsOneWidget);
    expect(find.byIcon(Icons.last_page), findsNothing);

    await tester.tap(find.text('LIVE'));
    await tester.pump();
    expect(sent.returns, 0);
    expect(sent.rewinds, 0);
  });

  testWidgets('a VOD has nothing', (tester) async {
    await pumpSlot(tester, const PlaybackRewindNone());
    expect(find.text('LIVE'), findsNothing);
    expect(find.byType(TextButton), findsNothing);
  });

  testWidgets("each of Rust's answers shows once, for a while", (tester) async {
    final answers = StreamController<RewindAnswer>.broadcast();
    addTearDown(answers.close);
    await pumpSlot(
      tester,
      const PlaybackRewindOffered(),
      answers: answers.stream,
    );
    expect(find.text('No recording.'), findsNothing);

    await answer(tester, answers, const RewindAnswer(message: 'No recording.'));
    expect(find.text('No recording.'), findsOneWidget);

    await tester.pump(const Duration(seconds: 4));
    expect(find.text('No recording.'), findsNothing);

    // A new state does not bring it back.
    await pumpSlot(
      tester,
      const PlaybackRewindLoading(),
      answers: answers.stream,
    );
    await tester.pump(const Duration(seconds: 1));
    expect(find.text('No recording.'), findsNothing);

    // The same message again is a new answer.
    await answer(tester, answers, const RewindAnswer(message: 'No recording.'));
    expect(find.text('No recording.'), findsOneWidget);
    await tester.pump(const Duration(seconds: 4));
  });

  testWidgets('a new message replaces the one showing', (tester) async {
    final answers = StreamController<RewindAnswer>.broadcast();
    addTearDown(answers.close);
    await pumpSlot(
      tester,
      const PlaybackRewindOffered(),
      answers: answers.stream,
    );

    await answer(tester, answers, const RewindAnswer(message: 'First.'));
    await tester.pump(const Duration(seconds: 3));
    await answer(tester, answers, const RewindAnswer(message: 'Second.'));
    expect(find.text('First.'), findsNothing);
    expect(find.text('Second.'), findsOneWidget);

    // Its own 4 s, not what was left of the first's.
    await tester.pump(const Duration(seconds: 2));
    expect(find.text('Second.'), findsOneWidget);
    await tester.pump(const Duration(seconds: 2));
    expect(find.text('Second.'), findsNothing);
  });
}
