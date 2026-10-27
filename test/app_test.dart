import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/app.dart';
import 'package:jitter/boot/boot_holder.dart';
import 'package:jitter/favorites/favorites_holder.dart';
import 'package:jitter/top/browse_holder.dart';

void main() {
  testWidgets('spins until booted', (tester) async {
    await tester.pumpWidget(
      JitterApp(
        boot: BootHolder(),
        browse: BrowseHolder(),
        favorites: FavoritesHolder(),
      ),
    );

    expect(find.byType(CircularProgressIndicator), findsOneWidget);
  });

  testWidgets('shows a boot failure with Retry', (tester) async {
    final boot = BootHolder();
    await tester.pumpWidget(
      JitterApp(
        boot: boot,
        browse: BrowseHolder(),
        favorites: FavoritesHolder(),
      ),
    );
    boot.fail('Storage failed: disk full');
    await tester.pump();

    expect(find.text('Storage failed: disk full'), findsOneWidget);
    expect(find.text('Retry'), findsOneWidget);
  });

  testWidgets('narrow shell has a bottom bar', (tester) async {
    tester.view.physicalSize = const Size(400, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      JitterApp(
        boot: BootHolder()..booted(),
        browse: BrowseHolder(),
        favorites: FavoritesHolder(),
      ),
    );

    expect(find.byType(NavigationBar), findsOneWidget);
    expect(find.byType(NavigationRail), findsNothing);
    expect(find.text('Top'), findsOneWidget);
    expect(find.text('Favorites'), findsOneWidget);
  });

  testWidgets('wide shell has a side rail', (tester) async {
    tester.view.physicalSize = const Size(1200, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      JitterApp(
        boot: BootHolder()..booted(),
        browse: BrowseHolder(),
        favorites: FavoritesHolder(),
      ),
    );

    expect(find.byType(NavigationRail), findsOneWidget);
    expect(find.byType(NavigationBar), findsNothing);
    await tester.tap(find.text('Favorites'));
    await tester.pump();

    final rail = tester.widget<NavigationRail>(find.byType(NavigationRail));
    expect(rail.selectedIndex, 1);
  });
}
