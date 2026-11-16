import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/chat/chat_holder.dart';
import 'package:jitter/player/playback_holder.dart';
import 'package:jitter/player/playback_loading.dart';
import 'package:jitter/player/player_page.dart';
import 'package:jitter/player/player_sends.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:material_ui/material_ui.dart';

import 'fake_player_engine.dart';

const _target = PlaybackTargetLive(login: 'somebody');

void main() {
  late List<Object> sent;

  setUp(() => sent = []);

  Future<void> pumpPage(WidgetTester tester) => tester.pumpWidget(
    MaterialApp(
      home: PlayerPage(
        target: _target,
        title: 'Title',
        playback: PlaybackHolder(),
        player: FakePlayerEngine(),
        chat: ChatHolder(),
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
}
