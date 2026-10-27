import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/chat/chat_holder.dart';
import 'package:jitter/chat/chat_panel.dart';
import 'package:jitter/src/bindings/bindings.dart';

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
}
