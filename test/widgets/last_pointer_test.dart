import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/widgets/last_pointer.dart';

/// Pumps a box that says whether the last pointer was a touch.
Future<void> pumpBox(WidgetTester tester) => tester.pumpWidget(
  Directionality(
    textDirection: TextDirection.ltr,
    child: LastPointer(
      child: Builder(
        builder: (context) =>
            Text(LastPointer.isTouch(context) ? 'touch' : 'mouse'),
      ),
    ),
  ),
);

void main() {
  testWidgets('guesses touch on a phone, a mouse elsewhere', (tester) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.android;
    await pumpBox(tester);
    expect(find.text('touch'), findsOneWidget);

    debugDefaultTargetPlatformOverride = TargetPlatform.linux;
    await tester.pumpWidget(const SizedBox());
    await pumpBox(tester);
    expect(find.text('mouse'), findsOneWidget);
    debugDefaultTargetPlatformOverride = null;
  });

  testWidgets('follows the last pointer: a touch, then a mouse', (
    tester,
  ) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.linux;
    await pumpBox(tester);

    await tester.tapAt(const Offset(10, 10));
    await tester.pump();
    expect(find.text('touch'), findsOneWidget);

    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await mouse.addPointer(location: const Offset(10, 10));
    await mouse.moveTo(const Offset(20, 20));
    await tester.pump();
    expect(find.text('mouse'), findsOneWidget);
    await mouse.removePointer();
    debugDefaultTargetPlatformOverride = null;
  });
}
