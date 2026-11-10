import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/app.dart';
import 'package:jitter/backup/backup_holder.dart';
import 'package:jitter/boot/boot_holder.dart';
import 'package:jitter/categories/categories_holder.dart';
import 'package:jitter/categories/categories_page.dart';
import 'package:jitter/categories/category_holder.dart';
import 'package:jitter/channel/channel_holder.dart';
import 'package:jitter/chat/chat_holder.dart';
import 'package:jitter/favorites/favorites_holder.dart';
import 'package:jitter/favorites/favorites_page.dart';
import 'package:jitter/history/history_holder.dart';
import 'package:jitter/logs/logs_holder.dart';
import 'package:jitter/player/media_kit_engine.dart';
import 'package:jitter/player/playback_holder.dart';
import 'package:jitter/search/search_holder.dart';
import 'package:jitter/settings/settings_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:jitter/top/browse_holder.dart';
import 'package:jitter/top/top_page.dart';

JitterApp _app(BootHolder boot, {SettingsHolder? settings}) => JitterApp(
  boot: boot,
  browse: BrowseHolder(),
  categories: CategoriesHolder(),
  category: CategoryHolder(),
  favorites: FavoritesHolder(),
  history: HistoryHolder(),
  channel: ChannelHolder(),
  playback: PlaybackHolder(),
  player: MediaKitEngine(),
  chat: ChatHolder(),
  search: SearchHolder(),
  settings: settings ?? SettingsHolder(),
  backup: BackupHolder(),
  logs: LogsHolder(),
);

SettingsUpdateSnapshot _settings(AppTheme theme) => SettingsUpdateSnapshot(
  theme: theme,
  animatedEmotes: true,
  autoLoadChat: true,
  startQuality: StartQuality.source,
  startQualities: const [],
  thirdPartyEmotes: true,
  backgroundAudio: true,
  version: '0.1.0',
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

  testWidgets('very wide shell shows Top, Categories and Favorites as Home', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(3440, 1440);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(_app(BootHolder()..booted()));

    final rail = find.byType(NavigationRail);
    for (final label in ['Home', 'History', 'Search', 'Settings']) {
      expect(find.descendant(of: rail, matching: find.text(label)), findsOne);
    }
    for (final label in ['Top', 'Categories', 'Favorites']) {
      expect(
        find.descendant(of: rail, matching: find.text(label)),
        findsNothing,
      );
    }
    expect(find.byType(TopPage), findsOneWidget);
    expect(find.byType(CategoriesPage), findsOneWidget);
    expect(find.byType(FavoritesPage), findsOneWidget);
  });

  testWidgets('a page keeps its state across the Home width', (tester) async {
    tester.view.physicalSize = const Size(1920, 1080);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(_app(BootHolder()..booted()));
    await tester.tap(find.text('Favorites'));
    await tester.pump();
    final before = tester.state(find.byType(FavoritesPage));

    tester.view.physicalSize = const Size(3440, 1440);
    await tester.pump();
    expect(tester.state(find.byType(FavoritesPage)), same(before));

    tester.view.physicalSize = const Size(1920, 1080);
    await tester.pump();
    expect(tester.state(find.byType(FavoritesPage)), same(before));
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
