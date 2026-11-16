import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/widgets/loading_mark.dart';
import 'package:material_ui/material_ui.dart';

void main() {
  testWidgets('shows an hourglass and "Loading…"', (tester) async {
    await tester.pumpWidget(const MaterialApp(home: LoadingMark()));

    expect(find.text('Loading…'), findsOneWidget);
    expect(find.byIcon(Icons.hourglass_empty), findsOneWidget);
  });

  testWidgets('schedules no frames, so it never animates', (tester) async {
    await tester.pumpWidget(const MaterialApp(home: LoadingMark()));
    // The first pump also runs the frame callbacks MaterialApp queues, which
    // leave a frame scheduled even for an empty app.
    await tester.pump();

    expect(tester.binding.hasScheduledFrame, isFalse);
  });
}
