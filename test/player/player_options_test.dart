import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/chat/chat_holder.dart';
import 'package:jitter/player/playback_holder.dart';
import 'package:jitter/player/player_options.dart';
import 'package:jitter/src/bindings/bindings.dart';

const _qualities = [
  QualityOption(name: 'source', label: '1080p60'),
  QualityOption(name: '720p', label: '720p'),
];

const _presets = [
  SleepPreset(minutes: 15, label: '15 minutes'),
  SleepPreset(minutes: 30, label: '30 minutes'),
];

PlaybackUpdateReady _ready({String? left}) => PlaybackUpdateReady(
  qualities: _qualities,
  selected: 'source',
  live: true,
  details: const PlaybackDetails(chapters: []),
  sleep: PlaybackSleep(presets: _presets, left: left),
);

void main() {
  late List<String> sent;
  late PlaybackHolder playback;
  late ChatHolder chat;

  setUp(() {
    sent = [];
    playback = PlaybackHolder();
    chat = ChatHolder(
      sendHide: () => sent.add('hide'),
      sendShow: () => sent.add('show'),
    );
  });

  /// The options in an app bar, [width] wide; what they do goes to [sent].
  Future<void> pump(WidgetTester tester, {required double width}) async {
    tester.view.physicalSize = Size(width, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          appBar: AppBar(
            actions: [
              PlayerOptions(
                playback: playback,
                chat: chat,
                onSelectQuality: (name) => sent.add('quality $name'),
                onArmSleep: (minutes) => sent.add('sleep $minutes'),
                onTurnOffSleep: () => sent.add('sleep off'),
                onToggleTheatre: () => sent.add('theatre'),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  group('narrow, a bottom sheet', () {
    testWidgets('shows each option with its current value', (tester) async {
      playback.apply(_ready());
      chat.apply(const ChatUpdateHidden());
      await pump(tester, width: 400);

      await tap(tester, find.byTooltip('Options'));

      expect(find.byType(BottomSheet), findsOneWidget);
      expect(find.text('1080p60'), findsOneWidget);
      expect(find.text('Off'), findsOneWidget);
      expect(find.text('Chat'), findsOneWidget);
      expect(find.text('Theatre mode'), findsOneWidget);
    });

    testWidgets('picks a quality in the same sheet, then closes', (
      tester,
    ) async {
      playback.apply(_ready());
      await pump(tester, width: 400);

      await tap(tester, find.byTooltip('Options'));
      await tap(tester, find.text('Quality'));
      expect(find.text('Theatre mode'), findsNothing);
      await tap(tester, find.text('720p'));

      expect(sent, ['quality 720p']);
      expect(find.byType(BottomSheet), findsNothing);
    });

    testWidgets('goes back from a sub-list to the main one', (tester) async {
      playback.apply(_ready());
      await pump(tester, width: 400);

      await tap(tester, find.byTooltip('Options'));
      await tap(tester, find.text('Sleep timer'));
      await tap(tester, find.byIcon(Icons.arrow_back));

      expect(find.text('Theatre mode'), findsOneWidget);
    });

    testWidgets('an armed timer shows its time left and turns off', (
      tester,
    ) async {
      playback.apply(_ready(left: '23 min left'));
      await pump(tester, width: 400);

      await tap(tester, find.byTooltip('Options (sleep timer: 23 min left)'));
      expect(find.text('23 min left'), findsOneWidget);
      await tap(tester, find.text('Sleep timer'));
      await tap(tester, find.text('Turn off timer'));

      expect(sent, ['sleep off']);
    });

    testWidgets('the chat switch toggles and keeps the sheet open', (
      tester,
    ) async {
      chat.apply(const ChatUpdateHidden());
      await pump(tester, width: 400);

      await tap(tester, find.byTooltip('Options'));
      await tap(tester, find.text('Chat'));

      expect(sent, ['show']);
      expect(find.byType(BottomSheet), findsOneWidget);
    });

    testWidgets('before playback is ready only theatre is offered', (
      tester,
    ) async {
      await pump(tester, width: 400);

      await tap(tester, find.byTooltip('Options'));

      expect(find.text('Quality'), findsNothing);
      expect(find.text('Sleep timer'), findsNothing);
      expect(find.text('Chat'), findsNothing);
      await tap(tester, find.text('Theatre mode'));
      expect(sent, ['theatre']);
    });
  });

  group('wide, a menu', () {
    testWidgets('picks a quality from its submenu', (tester) async {
      playback.apply(_ready());
      await pump(tester, width: 1000);

      await tap(tester, find.byTooltip('Options'));
      expect(find.byType(BottomSheet), findsNothing);
      await tap(tester, find.text('Quality'));
      await tap(tester, find.text('720p'));

      expect(sent, ['quality 720p']);
    });

    testWidgets('arms a sleep timer from its submenu', (tester) async {
      playback.apply(_ready());
      await pump(tester, width: 1000);

      await tap(tester, find.byTooltip('Options'));
      await tap(tester, find.text('Sleep timer'));
      expect(find.text('Turn off timer'), findsNothing);
      await tap(tester, find.text('30 minutes'));

      expect(sent, ['sleep 30']);
    });

    testWidgets('hides a shown chat', (tester) async {
      chat.apply(
        const ChatUpdateSnapshot(
          messages: [],
          status: ChatStatus(modes: []),
        ),
      );
      await pump(tester, width: 1000);

      await tap(tester, find.byTooltip('Options'));
      await tap(tester, find.text('Chat'));

      expect(sent, ['hide']);
    });

    testWidgets('toggles theatre', (tester) async {
      await pump(tester, width: 1000);

      await tap(tester, find.byTooltip('Options'));
      await tap(tester, find.text('Theatre mode'));

      expect(sent, ['theatre']);
    });
  });
}
