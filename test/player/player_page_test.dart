import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/chat/chat_holder.dart';
import 'package:jitter/chat/chat_panel.dart';
import 'package:jitter/player/desktop_controls.dart';
import 'package:jitter/player/playback_holder.dart';
import 'package:jitter/player/playback_loading.dart';
import 'package:jitter/player/player_engine.dart';
import 'package:jitter/player/player_page.dart';
import 'package:jitter/player/player_sends.dart';
import 'package:jitter/player/video_frame.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:material_ui/material_ui.dart';

import 'fake_player_engine.dart';

const _target = PlaybackTargetLive(login: 'somebody');

const _vod = PlaybackUpdateReady(
  qualities: [QualityOption(name: 'source', label: 'Source')],
  selected: 'source',
  live: false,
  details: PlaybackDetails(chapters: []),
  sleep: PlaybackSleep(presets: []),
  rewind: PlaybackRewindNone(),
);

const _mediaKit = MethodChannel('com.alexmercerind/media_kit_video');

/// A player that records the seeks the keys ask for.
class _SeekingEngine extends FakePlayerEngine {
  final seeks = <Duration>[];

  @override
  Future<void> seekBy(Duration by) async => seeks.add(by);
}

/// The media_kit calls the desktop fullscreen makes, by method, in order.
List<String> recordNative(WidgetTester tester) {
  final calls = <String>[];
  tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(_mediaKit, (
    call,
  ) async {
    calls.add(call.method);
    return null;
  });
  return calls;
}

/// A chat shown, whose hide and show sends are recorded into [chatSends].
ChatHolder _shownChat(List<String> chatSends) =>
    ChatHolder(
      sendTheme: (_) {},
      sendHide: () => chatSends.add('hide'),
      sendShow: () => chatSends.add('show'),
    )..apply(
      const ChatUpdateSnapshot(
        messages: [],
        status: ChatStatus(modes: []),
      ),
    );

/// Runs [body] as on desktop, where the page shows its mouse controls. The
/// override is reset when the body ends: flutter_test checks it is unset.
Future<void> _onDesktop(Future<void> Function() body) async {
  debugDefaultTargetPlatformOverride = TargetPlatform.linux;
  try {
    await body();
  } finally {
    debugDefaultTargetPlatformOverride = null;
  }
}

/// Presses [key] and lets the frame it causes, and the focus request after
/// it, be built.
Future<void> press(WidgetTester tester, LogicalKeyboardKey key) async {
  await tester.sendKeyEvent(key);
  await tester.pump();
  await tester.pump();
}

void main() {
  late List<Object> sent;

  setUp(() => sent = []);

  Future<void> pumpPage(
    WidgetTester tester, {
    PlaybackHolder? playback,
    PlayerEngine? player,
    ChatHolder? chat,
  }) => tester.pumpWidget(
    MaterialApp(
      home: PlayerPage(
        target: _target,
        title: 'Title',
        playback: playback ?? PlaybackHolder(),
        player: player ?? FakePlayerEngine(),
        chat: chat ?? ChatHolder(),
        onOpenChannel: (_) {},
        sends: PlayerSends.to(sent.add),
      ),
    ),
  );

  testWidgets('opening the page asks Rust to play its target', (tester) async {
    await pumpPage(tester);

    final plays = sent.whereType<Play>().toList();
    expect(plays, hasLength(1));
    expect(plays.single.target, _target);
    expect(find.byType(PlaybackLoading), findsOneWidget);

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('the page reports the position every second', (tester) async {
    await pumpPage(tester);

    await tester.pump(const Duration(seconds: 1));

    expect(sent.whereType<PlayerPosition>(), isNotEmpty);

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('leaving the page tells Rust', (tester) async {
    await pumpPage(tester);

    await tester.pumpWidget(const SizedBox());

    expect(sent.last, isA<LeavePlayer>());
  });

  group('fullscreen', () {
    testWidgets(
      'f fills the window with the video and keeps its frame',
      (tester) => _onDesktop(() async {
        final calls = recordNative(tester);
        final chatSends = <String>[];
        await pumpPage(
          tester,
          playback: PlaybackHolder()..apply(_vod),
          chat: _shownChat(chatSends),
        );
        expect(find.byType(AppBar), findsOneWidget);
        expect(find.byType(ChatPanel), findsOneWidget);
        expect(find.byKey(const Key('fake surface')), findsOneWidget);
        expect(find.byType(DesktopControls), findsOneWidget);
        final frame = tester.state(find.byType(VideoFrame));

        await press(tester, LogicalKeyboardKey.keyF);

        expect(find.byType(AppBar), findsNothing);
        expect(find.byType(ChatPanel), findsNothing);
        expect(find.byKey(const Key('fake surface')), findsOneWidget);
        expect(tester.state(find.byType(VideoFrame)), same(frame));
        expect(calls, contains('Utils.EnterNativeFullscreen'));

        await press(tester, LogicalKeyboardKey.keyF);

        expect(find.byType(AppBar), findsOneWidget);
        expect(find.byType(ChatPanel), findsOneWidget);

        await tester.pumpWidget(const SizedBox());
      }),
    );

    testWidgets(
      'Back in theatre and fullscreen leaves fullscreen only',
      (tester) => _onDesktop(() async {
        recordNative(tester);
        final chatSends = <String>[];
        await pumpPage(
          tester,
          playback: PlaybackHolder()..apply(_vod),
          chat: _shownChat(chatSends),
        );

        await press(tester, LogicalKeyboardKey.keyT);
        await press(tester, LogicalKeyboardKey.keyF);
        await tester.binding.handlePopRoute();
        await tester.pump();

        expect(find.byType(ChatPanel), findsOneWidget);
        expect(find.byType(AppBar), findsNothing);

        await tester.pumpWidget(const SizedBox());
      }),
    );

    testWidgets(
      't and c do nothing in fullscreen',
      (tester) => _onDesktop(() async {
        recordNative(tester);
        final chatSends = <String>[];
        await pumpPage(
          tester,
          playback: PlaybackHolder()..apply(_vod),
          chat: _shownChat(chatSends),
        );

        // Positive control: c reaches the chat out of fullscreen.
        await press(tester, LogicalKeyboardKey.keyC);
        expect(chatSends, ['hide']);

        await press(tester, LogicalKeyboardKey.keyF);
        await press(tester, LogicalKeyboardKey.keyT);
        await press(tester, LogicalKeyboardKey.keyC);
        expect(chatSends, ['hide']);

        await press(tester, LogicalKeyboardKey.keyF);
        expect(find.byType(AppBar), findsOneWidget);
        expect(find.byType(ChatPanel), findsOneWidget);

        await tester.pumpWidget(const SizedBox());
      }),
    );

    testWidgets(
      'after f the arrow keys reach the controls',
      (tester) => _onDesktop(() async {
        recordNative(tester);
        final engine = _SeekingEngine();
        await pumpPage(
          tester,
          playback: PlaybackHolder()..apply(_vod),
          player: engine,
          chat: _shownChat([]),
        );

        await press(tester, LogicalKeyboardKey.keyF);
        await press(tester, LogicalKeyboardKey.arrowRight);

        expect(engine.seeks, [const Duration(seconds: 10)]);

        await tester.pumpWidget(const SizedBox());
      }),
    );
  });
}
