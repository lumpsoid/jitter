import 'dart:async';

import 'package:flutter/material.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:rinf/rinf.dart';

/// The settings as Rust last sent them. Lives for the app's lifetime: the
/// app reads its theme, the settings page the rest. A change is sent to
/// Rust, which stores it and answers with the settings as they now are.
class SettingsHolder extends ChangeNotifier {
  new({this.send = _send});

  /// Sends `ChangeSetting`; tests without Rust replace it.
  final void Function(ChangeSetting) send;

  StreamSubscription<RustSignalPack<SettingsUpdate>>? _subscription;
  final _failures = StreamController<String>.broadcast(sync: true);

  /// One message per change Rust could not store, for a snackbar each.
  /// Not state: the settings are as last sent.
  Stream<String> get failures => _failures.stream;

  /// Null until the first snapshot arrives.
  SettingsUpdateSnapshot? get settings => _settings;
  SettingsUpdateSnapshot? _settings;

  /// The theme the app draws in; the platform's until the settings arrive.
  ThemeMode get themeMode => switch (_settings?.theme) {
    null || AppTheme.system => ThemeMode.system,
    AppTheme.light => ThemeMode.light,
    AppTheme.dark => ThemeMode.dark,
  };

  /// Subscribes to Rust's updates, then asks for the current settings.
  void connect() {
    _subscription = SettingsUpdate.rustSignalStream.listen(
      (pack) => apply(pack.message),
    );
    const SettingsReady().sendSignalToRust();
  }

  void chooseTheme(AppTheme theme) => _change(SettingChangeTheme(theme: theme));

  void chooseStartQuality(StartQuality quality) =>
      _change(SettingChangeStartQuality(quality: quality));

  void setAnimatedEmotes({required bool on}) =>
      _change(SettingChangeAnimatedEmotes(on: on));

  void setAutoLoadChat({required bool on}) =>
      _change(SettingChangeAutoLoadChat(on: on));

  void setThirdPartyEmotes({required bool on}) =>
      _change(SettingChangeThirdPartyEmotes(on: on));

  void _change(SettingChange change) => send(ChangeSetting(change: change));

  /// Folds one update in: a snapshot replaces the settings; a failure
  /// leaves them alone and goes out on [failures].
  void apply(SettingsUpdate update) {
    switch (update) {
      case SettingsUpdateSnapshot():
        _settings = update;
        notifyListeners();
      case SettingsUpdateFailed(:final message):
        _failures.add(message);
    }
  }

  @override
  void dispose() {
    unawaited(_subscription?.cancel());
    unawaited(_failures.close());
    super.dispose();
  }
}

void _send(ChangeSetting signal) => signal.sendSignalToRust();
