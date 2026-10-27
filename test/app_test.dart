import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/app.dart';
import 'package:jitter/boot/boot_holder.dart';
import 'package:jitter/categories/categories_holder.dart';
import 'package:jitter/categories/category_holder.dart';
import 'package:jitter/channel/channel_holder.dart';
import 'package:jitter/chat/chat_holder.dart';
import 'package:jitter/favorites/favorites_holder.dart';
import 'package:jitter/player/media_kit_engine.dart';
import 'package:jitter/player/playback_holder.dart';
import 'package:jitter/top/browse_holder.dart';

JitterApp _app(BootHolder boot) => JitterApp(
  boot: boot,
  browse: BrowseHolder(),
  categories: CategoriesHolder(),
  category: CategoryHolder(),
  favorites: FavoritesHolder(),
  channel: ChannelHolder(),
  playback: PlaybackHolder(),
  player: MediaKitEngine(),
  chat: ChatHolder(),
);

void main() {
  testWidgets('spins until booted', (tester) async {
    await tester.pumpWidget(_app(BootHolder()));

    expect(find.byType(CircularProgressIndicator), findsOneWidget);
  });

  testWidgets('shows a boot failure with Retry', (tester) async {
    final boot = BootHolder();
    await tester.pumpWidget(_app(boot));
    boot.fail('Storage failed: disk full');
    await tester.pump();

    expect(find.text('Storage failed: disk full'), findsOneWidget);
    expect(find.text('Retry'), findsOneWidget);
  });

  testWidgets('narrow shell has a bottom bar', (tester) async {
    tester.view.physicalSize = const Size(400, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(_app(BootHolder()..booted()));

    expect(find.byType(NavigationBar), findsOneWidget);
    expect(find.byType(NavigationRail), findsNothing);
    expect(find.text('Top'), findsOneWidget);
    expect(find.text('Categories'), findsOneWidget);
    expect(find.text('Favorites'), findsOneWidget);
  });

  testWidgets('wide shell has a side rail', (tester) async {
    tester.view.physicalSize = const Size(1200, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(_app(BootHolder()..booted()));

    expect(find.byType(NavigationRail), findsOneWidget);
    expect(find.byType(NavigationBar), findsNothing);
    await tester.tap(find.text('Favorites'));
    await tester.pump();

    final rail = tester.widget<NavigationRail>(find.byType(NavigationRail));
    expect(rail.selectedIndex, 2);
  });
}
