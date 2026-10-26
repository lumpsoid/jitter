import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/main.dart';

void main() {
  testWidgets('shows the app title', (tester) async {
    await tester.pumpWidget(const JitterApp());

    expect(find.text('Jitter'), findsOneWidget);
  });
}
