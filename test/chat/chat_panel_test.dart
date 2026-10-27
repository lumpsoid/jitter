import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/chat/chat_holder.dart';
import 'package:jitter/chat/chat_panel.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:jitter/widgets/focus_ring.dart';

void main() {
  testWidgets('shows the status lines and the messages', (tester) async {
    final themes = <bool>[];
    final chat = ChatHolder(sendTheme: (signal) => themes.add(signal.dark))
      ..apply(
        const ChatUpdateSnapshot(
          messages: [
            ChatLine(
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
          ],
          status: ChatStatus(
            banner: 'Connecting to chat…',
            modes: ['Emote-only'],
            emoteHint: 'Fewer emotes: BTTV did not load',
          ),
        ),
      );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: ChatPanel(chat: chat)),
      ),
    );

    expect(find.text('Connecting to chat…'), findsOneWidget);
    expect(find.text('Emote-only'), findsOneWidget);
    expect(find.text('Fewer emotes: BTTV did not load'), findsOneWidget);
    expect(find.text('www.a.b'), findsOneWidget);
    expect(themes, [false]);
  });

  testWidgets('shows the connection bar below the messages, modes above', (
    tester,
  ) async {
    final chat = ChatHolder(sendTheme: (_) {})
      ..apply(
        const ChatUpdateSnapshot(
          messages: [
            ChatLine(
              id: 'a',
              name: 'alice',
              color: 0xFFFF0000,
              action: false,
              deleted: false,
              fragments: [ChatFragmentText(text: 'hi')],
            ),
          ],
          status: ChatStatus(
            banner: 'Connecting to chat…',
            modes: ['Emote-only'],
          ),
        ),
      );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: ChatPanel(chat: chat)),
      ),
    );

    final panel = tester.getRect(find.byType(ChatPanel));
    final banner = tester.getRect(find.text('Connecting to chat…'));
    final message = tester.getRect(find.textContaining('alice'));
    final modes = tester.getRect(find.text('Emote-only'));
    expect(banner.top, greaterThan(message.bottom));
    expect(panel.bottom - banner.bottom, lessThan(16));
    expect(modes.bottom, lessThan(message.top));
  });

  testWidgets('status lines are live regions, messages are not', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    final chat = ChatHolder(sendTheme: (_) {})
      ..apply(
        const ChatUpdateSnapshot(
          messages: [
            ChatLine(
              id: 'a',
              name: 'alice',
              color: 0xFFFF0000,
              action: false,
              deleted: false,
              fragments: [ChatFragmentText(text: 'hi')],
            ),
          ],
          status: ChatStatus(
            banner: 'Reconnecting to chat…',
            modes: ['Emote-only'],
            notice: 'This room is in slow mode.',
          ),
        ),
      );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: ChatPanel(chat: chat)),
      ),
    );

    for (final line in [
      'Reconnecting to chat…',
      'Emote-only',
      'This room is in slow mode.',
    ]) {
      expect(
        tester.getSemantics(find.bySemanticsLabel(line)),
        isSemantics(label: line, isLiveRegion: true),
      );
    }
    expect(
      tester.getSemantics(find.bySemanticsLabel(RegExp('alice'))),
      isSemantics(isLiveRegion: false),
    );
    semantics.dispose();
  });

  testWidgets('a link is announced as one, and Tab then Enter opens it', (
    tester,
  ) async {
    final opened = <Object?>[];
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      const MethodChannel('plugins.flutter.io/url_launcher'),
      (call) async {
        opened.add((call.arguments as Map<Object?, Object?>)['url']);
        return true;
      },
    );
    addTearDown(
      () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        const MethodChannel('plugins.flutter.io/url_launcher'),
        null,
      ),
    );
    final semantics = tester.ensureSemantics();
    final chat = ChatHolder(sendTheme: (_) {})
      ..apply(
        const ChatUpdateSnapshot(
          messages: [
            ChatLine(
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
          ],
          status: ChatStatus(modes: []),
        ),
      );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: ChatPanel(chat: chat)),
      ),
    );

    expect(
      tester.getSemantics(find.text('www.a.b')),
      matchesSemantics(
        label: 'www.a.b',
        isLink: true,
        isFocusable: true,
        hasTapAction: true,
        hasFocusAction: true,
      ),
    );

    // The messages first, then the link in them.
    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump();
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pump();

    expect(opened, ['https://www.a.b']);
    semantics.dispose();
  });

  testWidgets('marks a deleted message as deleted, not only faded', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    final chat = ChatHolder(sendTheme: (_) {})
      ..apply(
        const ChatUpdateSnapshot(
          messages: [
            ChatLine(
              id: 'a',
              name: 'alice',
              color: 0xFFFF0000,
              action: false,
              deleted: false,
              fragments: [ChatFragmentText(text: 'hi')],
            ),
            ChatLine(
              id: 'b',
              name: 'bob',
              color: 0xFF0000FF,
              action: false,
              deleted: false,
              fragments: [
                ChatFragmentText(text: 'bad words '),
                ChatFragmentLink(text: 'x.y', url: 'https://x.y'),
              ],
            ),
          ],
          status: ChatStatus(modes: []),
        ),
      )
      ..apply(const ChatUpdateMarked(ids: ['b']));
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: ChatPanel(chat: chat)),
      ),
    );

    expect(find.textContaining('(deleted)'), findsOneWidget);
    expect(find.textContaining('alice'), findsOneWidget);
    expect(find.bySemanticsLabel(RegExp('bob.*deleted.*bad words')), findsOne);
    expect(find.bySemanticsLabel(RegExp('alice.*deleted')), findsNothing);
    expect(find.byType(Opacity), findsNothing);

    final rich = tester.widget<RichText>(
      find.byWidgetPredicate(
        (widget) =>
            widget is RichText &&
            widget.text.toPlainText().contains('bad words'),
      ),
    );
    final struck = <String>[];
    rich.text.visitChildren((span) {
      final decoration = span.style?.decoration;
      if (span is TextSpan &&
          decoration != null &&
          decoration.contains(TextDecoration.lineThrough)) {
        struck.add(span.text ?? '');
      }
      return true;
    });
    expect(struck, contains('bad words '));
    final link = tester.widget<Text>(find.text('x.y')).style?.decoration;
    expect(link?.contains(TextDecoration.lineThrough), isTrue);
    expect(link?.contains(TextDecoration.underline), isTrue);
    semantics.dispose();
  });

  testWidgets('emotes and their fallback names grow with the text size', (
    tester,
  ) async {
    Future<({double emote, double fallback})> heightsAt(double scale) async {
      // A fresh tree, so no earlier failure carries over.
      await tester.pumpWidget(const SizedBox());
      await tester.pumpWidget(
        MaterialApp(
          home: MediaQuery(
            data: MediaQueryData(textScaler: TextScaler.linear(scale)),
            child: Scaffold(
              body: ChatPanel(
                chat: _chatWith([
                  ChatFragmentEmote(
                    // A fresh URL, or the cached failure skips the loading.
                    emote: EmoteImage(name: 'Kappa', url: 'https://e/$scale'),
                  ),
                ]),
              ),
            ),
          ),
        ),
      );
      // Still loading: the image holds its height.
      final emote = tester.getRect(find.byType(Image)).height;
      // Test HTTP fails, so the name stands in.
      await tester.pumpAndSettle();
      final fallback = tester.getRect(find.text('Kappa')).height;
      return (emote: emote, fallback: fallback);
    }

    final normal = await heightsAt(1);
    final large = await heightsAt(2);

    expect(large.emote, moreOrLessEquals(normal.emote * 2));
    expect(large.fallback, moreOrLessEquals(normal.fallback * 2));
  });

  testWidgets('an overlay emote reads as one image, its base', (tester) async {
    final semantics = tester.ensureSemantics();
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ChatPanel(
            chat: _chatWith(const [
              ChatFragmentText(text: 'hi '),
              ChatFragmentOverlay(
                base: EmoteImage(name: 'Base', url: 'https://e/b.png'),
                overlays: [
                  EmoteImage(name: 'Hat', url: 'https://e/h.png'),
                  EmoteImage(name: 'Rain', url: 'https://e/r.png'),
                ],
              ),
            ]),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(
      tester.getSemantics(find.bySemanticsLabel('Base')),
      matchesSemantics(label: 'Base', isImage: true),
    );
    expect(find.bySemanticsLabel(RegExp('Hat|Rain')), findsNothing);
    semantics.dispose();
  });

  testWidgets('the messages are a ringed Tab stop the keys scroll, pausing', (
    tester,
  ) async {
    final chat = ChatHolder(sendTheme: (_) {})
      ..apply(
        ChatUpdateSnapshot(
          messages: [for (var i = 0; i < 60; i++) _line('$i', 'message $i')],
          status: const ChatStatus(modes: []),
        ),
      );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: ChatPanel(chat: chat)),
      ),
    );
    double offset() =>
        tester.state<ScrollableState>(find.byType(Scrollable)).position.pixels;
    bool ringShown() => tester.widget<FocusRing>(find.byType(FocusRing)).shown;
    Future<void> press(LogicalKeyboardKey key) async {
      await tester.sendKeyEvent(key);
      await tester.pumpAndSettle();
    }

    expect(ringShown(), isFalse);
    await press(LogicalKeyboardKey.tab);
    expect(primaryFocus?.debugLabel, 'Chat messages');
    expect(ringShown(), isTrue);

    await press(LogicalKeyboardKey.arrowUp);
    expect(offset(), greaterThan(0));
    expect(find.text('Latest messages'), findsOneWidget);

    // Paused: a new message waits until the list is back at the bottom.
    chat.apply(ChatUpdateAppended(messages: [_line('new', 'newest')]));
    await tester.pump();
    expect(find.textContaining('newest'), findsNothing);

    final lineDown = offset();
    await press(LogicalKeyboardKey.pageUp);
    expect(offset(), greaterThan(lineDown));
    await press(LogicalKeyboardKey.home);
    final top = tester
        .state<ScrollableState>(find.byType(Scrollable))
        .position
        .maxScrollExtent;
    expect(offset(), top);

    await press(LogicalKeyboardKey.end);
    expect(offset(), 0);
    expect(find.text('Latest messages'), findsNothing);
    expect(find.textContaining('newest'), findsOneWidget);

    await press(LogicalKeyboardKey.arrowUp);
    await press(LogicalKeyboardKey.arrowDown);
    expect(offset(), 0);
    expect(find.text('Latest messages'), findsNothing);
  });

  testWidgets('Tab reaches "Latest messages" after the messages', (
    tester,
  ) async {
    final chat = ChatHolder(sendTheme: (_) {})
      ..apply(
        ChatUpdateSnapshot(
          messages: [for (var i = 0; i < 60; i++) _line('$i', 'message $i')],
          status: const ChatStatus(modes: []),
        ),
      );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: ChatPanel(chat: chat)),
      ),
    );
    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowUp);
    await tester.pumpAndSettle();

    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump();
    expect(
      primaryFocus?.context?.findAncestorWidgetOfExactType<ActionChip>(),
      isNotNull,
    );
    expect(tester.widget<FocusRing>(find.byType(FocusRing)).shown, isFalse);

    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();
    expect(find.text('Latest messages'), findsNothing);
  });

  testWidgets('a focused link rings itself, not the messages', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ChatPanel(
            chat: _chatWith(const [
              ChatFragmentLink(text: 'www.a.b', url: 'https://www.a.b'),
            ]),
          ),
        ),
      ),
    );

    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump();

    expect(
      primaryFocus?.context?.findAncestorWidgetOfExactType<InkWell>(),
      isNotNull,
    );
    expect(tester.widget<FocusRing>(find.byType(FocusRing)).shown, isFalse);
  });
}

ChatHolder _chatWith(List<ChatFragment> fragments) {
  return ChatHolder(sendTheme: (_) {})..apply(
    ChatUpdateSnapshot(
      messages: [
        ChatLine(
          id: 'a',
          name: 'alice',
          color: 0xFFFF0000,
          action: false,
          deleted: false,
          fragments: fragments,
        ),
      ],
      status: const ChatStatus(modes: []),
    ),
  );
}

ChatLine _line(String id, String text) => ChatLine(
  id: id,
  name: 'alice',
  color: 0xFFFF0000,
  action: false,
  deleted: false,
  fragments: [ChatFragmentText(text: text)],
);
