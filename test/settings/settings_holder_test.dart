import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/settings/settings_holder.dart';
import 'package:jitter/src/bindings/bindings.dart';

import 'fixtures.dart';

void main() {
  late SettingsHolder holder;
  late List<SettingChange> sent;
  late int notifications;

  setUp(() {
    sent = [];
    holder = SettingsHolder(send: (signal) => sent.add(signal.change));
    notifications = 0;
    holder.addListener(() => notifications++);
  });

  test('has no settings and follows the system theme until a snapshot', () {
    expect(holder.settings, isNull);
    expect(holder.themeMode, ThemeMode.system);
  });

  test('a snapshot replaces the settings and maps the theme', () {
    holder.apply(snapshot(theme: AppTheme.dark, autoLoadChat: false));
    expect(holder.themeMode, ThemeMode.dark);
    expect(holder.settings?.autoLoadChat, isFalse);
    holder.apply(snapshot(theme: AppTheme.light));
    expect(holder.themeMode, ThemeMode.light);
    holder.apply(snapshot());
    expect(holder.themeMode, ThemeMode.system);
    expect(notifications, 3);
  });

  test('a failure goes out once and leaves the settings alone', () async {
    holder.apply(snapshot(theme: AppTheme.dark));
    final failures = <String>[];
    holder.failures.listen(failures.add);
    holder.apply(const SettingsUpdateFailed(message: 'Storage failed: x'));
    expect(failures, ['Storage failed: x']);
    expect(holder.themeMode, ThemeMode.dark);
    expect(notifications, 1);
  });

  test('each change sends its setting and waits for Rust', () {
    holder
      ..chooseTheme(AppTheme.light)
      ..chooseStartQuality(StartQuality.p720)
      ..setAnimatedEmotes(on: false)
      ..setAutoLoadChat(on: false)
      ..setThirdPartyEmotes(on: true)
      ..setBackgroundAudio(on: false);
    expect(sent, const [
      SettingChangeTheme(theme: AppTheme.light),
      SettingChangeStartQuality(quality: StartQuality.p720),
      SettingChangeAnimatedEmotes(on: false),
      SettingChangeAutoLoadChat(on: false),
      SettingChangeThirdPartyEmotes(on: true),
      SettingChangeBackgroundAudio(on: false),
    ]);
    expect(holder.settings, isNull);
    expect(notifications, 0);
  });
}
