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
import 'package:jitter/search/search_holder.dart';
import 'package:jitter/settings/settings_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:jitter/top/browse_holder.dart';

JitterApp _app(BootHolder boot, {SettingsHolder? settings}) => JitterApp(
  boot: boot,
  browse: BrowseHolder(),
  categories: CategoriesHolder(),
  category: CategoryHolder(),
  favorites: FavoritesHolder(),
  channel: ChannelHolder(),
  playback: PlaybackHolder(),
  player: MediaKitEngine(),
  chat: ChatHolder(),
  search: SearchHolder(),
  settings: settings ?? SettingsHolder(),
);

SettingsUpdateSnapshot _settings(AppTheme theme) => SettingsUpdateSnapshot(
  theme: theme,
  animatedEmotes: true,
  autoLoadChat: true,
  startQuality: StartQuality.source,
  startQualities: const [],
  thirdPartyEmotes: true,
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
    expect(find.text('Search'), findsOneWidget);
    expect(find.text('Settings'), findsOneWidget);
  });

  testWidgets('follows the system theme until Rust says otherwise', (
    tester,
  ) async {
    final settings = SettingsHolder();
    await tester.pumpWidget(_app(BootHolder(), settings: settings));

    ThemeMode mode() =>
        tester.widget<MaterialApp>(find.byType(MaterialApp)).themeMode!;
    expect(mode(), ThemeMode.system);
    settings.apply(_settings(AppTheme.dark));
    await tester.pump();
    expect(mode(), ThemeMode.dark);
    settings.apply(_settings(AppTheme.light));
    await tester.pump();
    expect(mode(), ThemeMode.light);
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
