import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/chat/chat_holder.dart';
import 'package:jitter/chat/chat_panel.dart';
import 'package:jitter/player/player_layout.dart';
import 'package:jitter/player/player_shortcuts.dart';
import 'package:jitter/player/video_frame.dart';
import 'package:jitter/src/bindings/bindings.dart';

/// A chat with one message holding a link.
ChatHolder _chat() => ChatHolder(sendTheme: (_) {})
  ..apply(
    const ChatUpdateSnapshot(
      messages: [
        ChatItemMessage(
          line: ChatLine(
            id: 'a',
            name: 'alice',
            color: 0xFFFF0000,
            action: false,
            deleted: false,
            fragments: [
              ChatFragmentText(text: 'see '),
              ChatFragmentLink(text: 'www.a.b', url: 'https://www.a.b'),
            ],
          ),
        ),
      ],
      status: ChatStatus(banner: 'Connecting to chat…', modes: ['Emote-only']),
    ),
  );

/// Pumps the page as `PlayerPage` lays it out, at [width]: the shortcuts
/// around stand-ins for the app bar's controls, then the layout, with a
/// loading video and, if [chat], the chat. Returns the page keys pressed.
Future<List<String>> pumpLayout(
  WidgetTester tester, {
  required double width,
  bool chat = true,
}) async {
  tester.view.physicalSize = Size(width, 800);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final holder = _chat();
  final pressed = <String>[];
  await tester.pumpWidget(
    MaterialApp(
      home: PlayerShortcuts(
        onToggleChat: () {},
        onToggleMute: () {},
        onToggleFullscreen: () {},
        onTogglePlay: () => pressed.add('play'),
        onToggleTheatre: () {},
        child: Scaffold(
          appBar: AppBar(
            title: TextButton(onPressed: () {}, child: const Text('Title')),
            actions: [
              for (final name in ['Quality', 'Chat', 'Theatre'])
                TextButton(onPressed: () {}, child: Text(name)),
            ],
          ),
          body: PlayerLayout(
            video: const VideoFrame(childTakesFocus: false, child: SizedBox()),
            chat: chat ? ChatPanel(chat: holder) : null,
          ),
        ),
      ),
    ),
  );
  return pressed;
}

/// What has the focus: its debug label, else the text inside it.
String focused(WidgetTester tester) {
  final node = primaryFocus!;
  if (node.debugLabel case final label?) return label;
  final texts = find.descendant(
    of: find.byElementPredicate((e) => e == node.context),
    matching: find.byType(Text),
  );
  if (texts.evaluate().isNotEmpty) {
    return tester.widget<Text>(texts.first).data ?? '?';
  }
  return '?';
}

/// The stops Tab visits from the page's open, in order, [count] of them.
Future<List<String>> tabThrough(WidgetTester tester, int count) async {
  final stops = <String>[];
  for (var i = 0; i < count; i++) {
    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump();
    stops.add(focused(tester));
  }
  return stops;
}

const _order = [
  'Title',
  'Quality',
  'Chat',
  'Theatre',
  'Video frame',
  'Chat messages',
  'www.a.b',
  'Title',
];

void main() {
  testWidgets('Tab goes app bar, video, then chat with the chat beside', (
    tester,
  ) async {
    await pumpLayout(tester, width: 1200);
    final video = tester.getRect(find.byType(VideoFrame));
    final chat = tester.getRect(find.byType(ChatPanel));
    expect(chat.left, greaterThanOrEqualTo(video.right));

    expect(await tabThrough(tester, _order.length), _order);
  });

  testWidgets('Tab goes app bar, video, then chat with the chat below', (
    tester,
  ) async {
    await pumpLayout(tester, width: 600);
    final video = tester.getRect(find.byType(VideoFrame));
    final chat = tester.getRect(find.byType(ChatPanel));
    expect(chat.top, greaterThanOrEqualTo(video.bottom));

    expect(await tabThrough(tester, _order.length), _order);
  });

  testWidgets('without the chat Tab goes app bar then video', (tester) async {
    await pumpLayout(tester, width: 1200, chat: false);

    expect(await tabThrough(tester, 6), [
      'Title',
      'Quality',
      'Chat',
      'Theatre',
      'Video frame',
      'Title',
    ]);
  });

  testWidgets('the page keys work as it opens and on the loading video', (
    tester,
  ) async {
    final pressed = await pumpLayout(tester, width: 1200);

    await tester.sendKeyEvent(LogicalKeyboardKey.space);
    await tabThrough(tester, 5);
    expect(focused(tester), 'Video frame');
    await tester.sendKeyEvent(LogicalKeyboardKey.space);

    expect(pressed, ['play', 'play']);
  });
}
