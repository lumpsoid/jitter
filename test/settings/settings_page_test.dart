import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/settings/settings_holder.dart';
import 'package:jitter/settings/settings_page.dart';
import 'package:jitter/src/bindings/bindings.dart';

import 'fixtures.dart';

Future<void> pumpPage(WidgetTester tester, SettingsHolder holder) =>
    tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: SettingsPage(settings: holder)),
      ),
    );

Finder switchFor(String title) => find.widgetWithText(SwitchListTile, title);

bool switchValue(WidgetTester tester, String title) =>
    tester.widget<SwitchListTile>(switchFor(title)).value;

void main() {
  late SettingsHolder holder;
  late List<SettingChange> sent;

  setUp(() {
    sent = [];
    holder = SettingsHolder(send: (signal) => sent.add(signal.change));
  });

  testWidgets('spins until the settings arrive', (tester) async {
    await pumpPage(tester, holder);

    expect(find.byType(CircularProgressIndicator), findsOneWidget);
  });

  testWidgets('shows every setting as Rust sent it', (tester) async {
    holder.apply(
      snapshot(
        theme: AppTheme.dark,
        animatedEmotes: false,
        startQuality: StartQuality.p720,
      ),
    );
    await pumpPage(tester, holder);

    final theme = tester.widget<SegmentedButton<AppTheme>>(
      find.byType(SegmentedButton<AppTheme>),
    );
    expect(theme.selected, {AppTheme.dark});
    expect(switchValue(tester, 'Load chat automatically'), isTrue);
    expect(switchValue(tester, 'Animated emotes'), isFalse);
    expect(switchValue(tester, 'Third-party emotes'), isTrue);
    final group = tester.widget<RadioGroup<StartQuality>>(
      find.byType(RadioGroup<StartQuality>),
    );
    expect(group.groupValue, StartQuality.p720);
    for (final label in ['Source', '1080p', '720p', 'Audio only']) {
      expect(find.widgetWithText(RadioListTile<StartQuality>, label), findsOne);
    }
  });

  testWidgets('choosing a theme sends it', (tester) async {
    holder.apply(snapshot());
    await pumpPage(tester, holder);

    await tester.tap(find.text('Dark'));
    await tester.pump();

    expect(sent, const [SettingChangeTheme(theme: AppTheme.dark)]);
  });

  testWidgets('each switch sends its setting flipped', (tester) async {
    holder.apply(snapshot(autoLoadChat: false));
    await pumpPage(tester, holder);

    await tester.tap(switchFor('Load chat automatically'));
    await tester.tap(switchFor('Animated emotes'));
    await tester.tap(switchFor('Third-party emotes'));
    await tester.pump();

    expect(sent, const [
      SettingChangeAutoLoadChat(on: true),
      SettingChangeAnimatedEmotes(on: false),
      SettingChangeThirdPartyEmotes(on: false),
    ]);
  });

  testWidgets('choosing a start quality sends it', (tester) async {
    holder.apply(snapshot());
    await pumpPage(tester, holder);

    await tester.ensureVisible(find.text('Audio only'));
    await tester.pump();
    await tester.tap(find.text('Audio only'));
    await tester.pump();

    expect(sent, const [
      SettingChangeStartQuality(quality: StartQuality.audioOnly),
    ]);
  });

  testWidgets('the page changes only when Rust answers', (tester) async {
    holder.apply(snapshot());
    await pumpPage(tester, holder);

    await tester.tap(switchFor('Animated emotes'));
    await tester.pump();
    expect(switchValue(tester, 'Animated emotes'), isTrue);

    holder.apply(snapshot(animatedEmotes: false));
    await tester.pump();
    expect(switchValue(tester, 'Animated emotes'), isFalse);
  });

  testWidgets('a failed change shows as a snackbar', (tester) async {
    holder.apply(snapshot());
    await pumpPage(tester, holder);

    holder.apply(const SettingsUpdateFailed(message: 'Storage failed: full'));
    await tester.pump();

    expect(find.text('Storage failed: full'), findsOneWidget);
  });

  testWidgets('every control is labelled for screen readers', (tester) async {
    final semantics = tester.ensureSemantics();
    holder.apply(snapshot());
    await pumpPage(tester, holder);

    for (final label in [
      'Load chat automatically',
      'Animated emotes',
      'Third-party emotes',
    ]) {
      final control = find.bySemanticsLabel(RegExp('^$label'));
      expect(control, findsOneWidget, reason: label);
      expect(
        tester.getSemantics(control),
        isSemantics(hasToggledState: true, hasTapAction: true),
        reason: label,
      );
    }
    for (final label in ['System', 'Light', 'Dark']) {
      expect(find.bySemanticsLabel(label), findsOneWidget, reason: label);
    }
    for (final label in ['Appearance', 'Chat', 'Starting quality']) {
      expect(
        tester.getSemantics(find.bySemanticsLabel(label)),
        isSemantics(isHeader: true),
        reason: label,
      );
    }
    await tester.ensureVisible(find.text('Audio only'));
    await tester.pump();
    expect(
      tester.getSemantics(find.bySemanticsLabel('720p')),
      isSemantics(hasCheckedState: true, isInMutuallyExclusiveGroup: true),
    );
    semantics.dispose();
  });
}
