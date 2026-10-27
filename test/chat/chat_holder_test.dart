import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/chat/chat_holder.dart';
import 'package:jitter/chat/chat_mode.dart';
import 'package:jitter/src/bindings/bindings.dart';

ChatLine line(String id, {bool deleted = false}) => ChatLine(
  id: id,
  name: 'user$id',
  color: 0xFFFF0000,
  action: false,
  deleted: deleted,
  fragments: [ChatFragmentText(text: 'hi $id')],
);

const connecting = ChatStatus(banner: 'Connecting to chat…', modes: []);
const connected = ChatStatus(modes: ['Slow mode (30s)']);

void main() {
  late ChatHolder holder;
  late int notifications;

  List<String> ids() => [for (final m in holder.messages) m.line.id];
  List<String> deleted() => [
    for (final m in holder.messages)
      if (m.deleted) m.line.id,
  ];

  setUp(() {
    holder = ChatHolder();
    notifications = 0;
    holder.addListener(() => notifications++);
  });

  test('is off with no messages until a snapshot', () {
    expect(holder.mode, ChatMode.off);
    expect(holder.messages, isEmpty);
    expect(holder.status.banner, isNull);
  });

  test('a snapshot opens the chat and replaces messages and status', () {
    holder
      ..apply(ChatUpdateSnapshot(messages: [line('a')], status: connecting))
      ..apply(
        ChatUpdateSnapshot(
          messages: [line('b'), line('c', deleted: true)],
          status: connected,
        ),
      );

    expect(holder.mode, ChatMode.shown);
    expect(ids(), ['b', 'c']);
    expect(deleted(), ['c']);
    expect(holder.status, connected);
    expect(notifications, 2);
  });

  test('diffs apply in order', () {
    holder
      ..apply(const ChatUpdateSnapshot(messages: [], status: connecting))
      ..apply(ChatUpdateAppended(messages: [line('a'), line('b')]))
      ..apply(ChatUpdateAppended(messages: [line('c'), line('d')]))
      ..apply(const ChatUpdateEvicted(count: 1))
      ..apply(const ChatUpdateMarked(ids: ['b', 'd']))
      ..apply(ChatUpdateAppended(messages: [line('e')]))
      ..apply(const ChatUpdateEvicted(count: 2));

    expect(ids(), ['d', 'e']);
    expect(deleted(), ['d']);
    expect(notifications, 7);
  });

  test('a mark reaches entries a paused view still holds', () {
    holder.apply(ChatUpdateSnapshot(messages: [line('a')], status: connected));
    final paused = List.of(holder.messages);
    holder
      ..apply(ChatUpdateAppended(messages: [line('b')]))
      ..apply(const ChatUpdateMarked(ids: ['a']));

    expect(paused.single.deleted, isTrue);
  });

  test('marking an id no longer listed changes nothing', () {
    holder
      ..apply(ChatUpdateSnapshot(messages: [line('a')], status: connected))
      ..apply(const ChatUpdateMarked(ids: ['gone']));

    expect(deleted(), isEmpty);
  });

  test('clear removes every message and keeps the status', () {
    holder
      ..apply(
        ChatUpdateSnapshot(messages: [line('a'), line('b')], status: connected),
      )
      ..apply(const ChatUpdateCleared())
      ..apply(ChatUpdateAppended(messages: [line('c')]));

    expect(ids(), ['c']);
    expect(holder.status, connected);
  });

  test('a status update replaces the status and keeps the messages', () {
    holder
      ..apply(ChatUpdateSnapshot(messages: [line('a')], status: connecting))
      ..apply(const ChatUpdateStatus(status: connected));

    expect(holder.status, connected);
    expect(ids(), ['a']);
  });

  test('off closes the chat and drops its messages', () {
    holder
      ..apply(ChatUpdateSnapshot(messages: [line('a')], status: connected))
      ..apply(const ChatUpdateOff());

    expect(holder.mode, ChatMode.off);
    expect(holder.messages, isEmpty);
    expect(holder.status.modes, isEmpty);
  });

  test('a replay seek clears, loads, then refills from the new position', () {
    const loading = ChatStatus(banner: 'Loading chat…', modes: []);
    const replaying = ChatStatus(modes: []);
    holder
      ..apply(ChatUpdateSnapshot(messages: [line('a')], status: replaying))
      ..apply(const ChatUpdateCleared())
      ..apply(const ChatUpdateStatus(status: loading));

    expect(holder.messages, isEmpty);
    expect(holder.status.banner, 'Loading chat…');

    holder
      ..apply(ChatUpdateAppended(messages: [line('x'), line('y')]))
      ..apply(const ChatUpdateStatus(status: replaying));

    expect(holder.mode, ChatMode.shown);
    expect(ids(), ['x', 'y']);
    expect(holder.status.banner, isNull);
  });

  test('hidden closes the panel and drops its messages', () {
    holder
      ..apply(ChatUpdateSnapshot(messages: [line('a')], status: connected))
      ..apply(const ChatUpdateHidden());

    expect(holder.mode, ChatMode.hidden);
    expect(holder.messages, isEmpty);
    expect(holder.status.modes, isEmpty);
  });
}
