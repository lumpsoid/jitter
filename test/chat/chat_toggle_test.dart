import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/chat/chat_holder.dart';
import 'package:jitter/chat/chat_toggle.dart';
import 'package:jitter/src/bindings/bindings.dart';

void main() {
  late List<String> sent;

  Future<void> pump(WidgetTester tester, ChatHolder chat) => tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        appBar: AppBar(actions: [ChatToggle(chat: chat)]),
      ),
    ),
  );

  ChatHolder holder() => ChatHolder(
    sendHide: () => sent.add('hide'),
    sendShow: () => sent.add('show'),
  );

  setUp(() => sent = []);

  testWidgets('is absent while there is no chat', (tester) async {
    await pump(tester, holder()..apply(const ChatUpdateOff()));

    expect(find.byType(IconButton), findsNothing);
  });

  testWidgets('hides a shown chat', (tester) async {
    final chat = holder()
      ..apply(
        const ChatUpdateSnapshot(
          messages: [],
          status: ChatStatus(modes: []),
        ),
      );
    await pump(tester, chat);

    await tester.tap(find.byTooltip('Hide chat'));

    expect(sent, ['hide']);
  });

  testWidgets('shows a hidden chat', (tester) async {
    await pump(tester, holder()..apply(const ChatUpdateHidden()));

    await tester.tap(find.byTooltip('Show chat'));

    expect(sent, ['show']);
  });
}
