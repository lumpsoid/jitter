import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/channel/channel_description.dart';
import 'package:jitter/channel/channel_holder.dart';
import 'package:jitter/channel/channel_page.dart';
import 'package:jitter/favorites/favorites_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';

import 'fixtures.dart';

Future<void> pumpPage(
  WidgetTester tester,
  ChannelHolder channel, {
  ValueChanged<String>? onOpenVod,
  ValueChanged<String>? onOpenLive,
}) async {
  tester.view.physicalSize = const Size(600, 1200);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    MaterialApp(
      home: ChannelPage(
        login: 'xqc',
        channel: channel,
        favorites: FavoritesHolder(),
        onOpenVod: onOpenVod,
        onOpenLive: onOpenLive,
      ),
    ),
  );
}

void main() {
  testWidgets('spins until Rust answers', (tester) async {
    await pumpPage(tester, ChannelHolder());

    expect(find.text('xqc'), findsOneWidget);
    expect(find.byType(LinearProgressIndicator), findsOneWidget);
    expect(find.byType(CircularProgressIndicator), findsOneWidget);
  });

  testWidgets('an unknown channel says so', (tester) async {
    final channel = ChannelHolder()
      ..applyHeader(const ChannelUpdateNotFound(login: 'xqc'));
    await pumpPage(tester, channel);

    expect(find.text('No channel is called xqc.'), findsOneWidget);
    expect(find.byType(TabBar), findsNothing);
  });

  testWidgets('a failed lookup shows its message with Retry', (tester) async {
    final channel = ChannelHolder()
      ..applyHeader(
        const ChannelUpdateFailed(login: 'xqc', message: 'offline'),
      );
    await pumpPage(tester, channel);

    expect(find.text('offline'), findsOneWidget);
    expect(find.text('Retry'), findsOneWidget);
  });

  testWidgets('a found channel shows its header, star, tabs and VODs', (
    tester,
  ) async {
    final channel = ChannelHolder()
      ..applyHeader(found())
      ..applyVods(
        VodsUpdateSnapshot(
          state: VodsStateLoaded(
            vods: [vod('1', progress: 0.25), vod('2')],
            more: const VodsMoreIdle(),
          ),
        ),
      );
    await pumpPage(tester, channel);

    expect(find.text('xQc'), findsNWidgets(2));
    expect(find.text('Offline'), findsOneWidget);
    expect(find.byIcon(Icons.star_outline), findsOneWidget);
    expect(find.text('Archive'), findsOneWidget);
    expect(find.text('Highlights'), findsOneWidget);
    expect(find.text('Uploads'), findsOneWidget);
    expect(find.text('Title 1'), findsOneWidget);
    expect(find.text('1:02:03'), findsNWidgets(2));
    // Only the VOD that would resume has a bar.
    final bars = tester.widgetList<LinearProgressIndicator>(
      find.byType(LinearProgressIndicator),
    );
    expect([for (final b in bars) b.value], [0.25]);
  });

  testWidgets('a channel without a description shows none', (tester) async {
    final channel = ChannelHolder()
      ..applyHeader(found())
      ..applyVods(loadedVods(['1']));
    await pumpPage(tester, channel);

    expect(find.byType(ChannelDescription), findsNothing);
  });

  testWidgets('a short description shows whole, with no toggle', (
    tester,
  ) async {
    final channel = ChannelHolder()
      ..applyHeader(found(description: 'The juicer.'))
      ..applyVods(loadedVods(['1']));
    await pumpPage(tester, channel);

    expect(find.text('The juicer.'), findsOneWidget);
    expect(find.text('Show more'), findsNothing);
  });

  testWidgets('a long description is collapsed and expands', (tester) async {
    final long = List.filled(
      12,
      'The best at absolutely everything.',
    ).join('\n');
    final channel = ChannelHolder()
      ..applyHeader(found(description: long))
      ..applyVods(loadedVods(['1']));
    await pumpPage(tester, channel);

    Text text() => tester.widget<Text>(find.text(long));
    expect(text().maxLines, 3);
    expect(find.text('Show more'), findsOneWidget);

    await tester.tap(find.text('Show more'));
    await tester.pump();
    expect(text().maxLines, isNull);
    expect(find.text('Show less'), findsOneWidget);

    await tester.tap(find.text('Show less'));
    await tester.pump();
    expect(text().maxLines, 3);
    expect(find.text('Show more'), findsOneWidget);
  });

  testWidgets('tapping a VOD hands on its id', (tester) async {
    final opened = <String>[];
    final channel = ChannelHolder()
      ..applyHeader(found())
      ..applyVods(loadedVods(['1']));
    await pumpPage(tester, channel, onOpenVod: opened.add);
    await tester.tap(find.text('Title 1'));

    expect(opened, ['1']);
  });

  testWidgets('tapping the live stream hands on the login', (tester) async {
    final opened = <String>[];
    final channel = ChannelHolder()
      ..applyHeader(
        found(
          live: const ChannelLiveLive(
            title: 'Reacting',
            gameName: 'Just Chatting',
            viewersLabel: '42K viewers',
          ),
        ),
      )
      ..applyVods(loadedVods([]));
    await pumpPage(tester, channel, onOpenLive: opened.add);

    expect(find.text('LIVE'), findsOneWidget);
    expect(find.text('No videos here.'), findsOneWidget);
    await tester.tap(find.text('42K viewers · Just Chatting · Reacting'));

    expect(opened, ['xqc']);
  });

  testWidgets('a failed first page of VODs shows its message with Retry', (
    tester,
  ) async {
    final channel = ChannelHolder()
      ..applyHeader(found())
      ..applyVods(
        const VodsUpdateSnapshot(state: VodsStateFailed(message: 'timed out')),
      );
    await pumpPage(tester, channel);

    expect(find.text('timed out'), findsOneWidget);
    expect(find.text('Retry'), findsOneWidget);
  });
}
