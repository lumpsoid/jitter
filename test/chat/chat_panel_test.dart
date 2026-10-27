import 'package:flutter/material.dart';
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
}
