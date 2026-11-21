import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/channel/vod_tile_view.dart';
import 'package:material_ui/material_ui.dart';

import 'fixtures.dart';

Future<void> pumpTile(WidgetTester tester, {VoidCallback? onDownload}) =>
    tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: VodTileView(vod: vod('1'), onDownload: onDownload),
        ),
      ),
    );

void main() {
  testWidgets('a long press offers Download and hands it on', (tester) async {
    var downloads = 0;
    await pumpTile(tester, onDownload: () => downloads++);

    await tester.longPress(find.byType(VodTileView));
    await tester.pumpAndSettle();

    expect(find.text('Download'), findsOneWidget);

    await tester.tap(find.text('Download'));
    await tester.pumpAndSettle();

    expect(downloads, 1);
    expect(find.text('Download'), findsNothing);
  });

  testWidgets('the menu key offers Download', (tester) async {
    await pumpTile(tester, onDownload: () {});

    // The shortcut sits above the tile's InkWell, so the key must reach it
    // from inside: focus the title, which lies within the InkWell.
    Focus.of(tester.element(find.text('Title 1'))).requestFocus();
    await tester.pump();
    await tester.sendKeyEvent(LogicalKeyboardKey.contextMenu);
    await tester.pumpAndSettle();

    expect(find.text('Download'), findsOneWidget);
  });

  testWidgets('without onDownload a long press does nothing', (tester) async {
    await pumpTile(tester);

    await tester.longPress(find.byType(VodTileView));
    await tester.pumpAndSettle();

    expect(find.byType(BottomSheet), findsNothing);
  });
}
