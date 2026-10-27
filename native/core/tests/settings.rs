use jitter_core::fakes::FakeSettingsStore;
use jitter_core::models::{EmoteProvider, QualityPreference, Setting, Settings, ThemeChoice};
use jitter_core::settings::SettingsController;

fn changed() -> Settings {
    Settings {
        theme: ThemeChoice::Dark,
        animated_emotes: false,
        auto_load_chat: false,
        start_quality: QualityPreference::P720,
        third_party_emotes: false,
    }
}

#[test]
fn defaults_are_the_system_theme_the_source_and_everything_on() {
    let defaults = Settings::default();
    assert_eq!(defaults.theme, ThemeChoice::System);
    assert!(defaults.animated_emotes);
    assert!(defaults.auto_load_chat);
    assert_eq!(defaults.start_quality, QualityPreference::Source);
    assert!(defaults.third_party_emotes);
}

#[test]
fn starts_at_the_defaults_until_loaded() {
    let store = FakeSettingsStore::with_settings(changed());
    let mut c = SettingsController::new(Box::new(store));
    assert_eq!(c.state(), &Settings::default());
    assert_eq!(c.load(), Ok(()));
    assert_eq!(c.state(), &changed());
}

#[test]
fn a_failed_load_keeps_the_settings() {
    let store = FakeSettingsStore::with_settings(changed()).failing_loads();
    let mut c = SettingsController::new(Box::new(store));
    assert!(c.load().is_err());
    assert_eq!(c.state(), &Settings::default());
}

#[test]
fn set_changes_one_setting_and_stores_it() {
    let store = FakeSettingsStore::default();
    let mut c = SettingsController::new(Box::new(store.clone()));
    assert_eq!(
        c.set(Setting::StartQuality(QualityPreference::P480)),
        Ok(())
    );
    let expected = Settings {
        start_quality: QualityPreference::P480,
        ..Settings::default()
    };
    assert_eq!(c.state(), &expected);
    assert_eq!(store.settings(), expected);
}

#[test]
fn a_failed_write_changes_nothing() {
    let store = FakeSettingsStore::default().failing_writes();
    let mut c = SettingsController::new(Box::new(store.clone()));
    assert!(c.set(Setting::Theme(ThemeChoice::Light)).is_err());
    assert_eq!(c.state(), &Settings::default());
    assert_eq!(store.settings(), Settings::default());
}

#[test]
fn with_changes_only_its_own_setting() {
    let base = Settings::default();
    assert_eq!(
        base.with(Setting::Theme(ThemeChoice::Dark)).theme,
        ThemeChoice::Dark
    );
    assert!(!base.with(Setting::AnimatedEmotes(false)).animated_emotes);
    assert!(!base.with(Setting::AutoLoadChat(false)).auto_load_chat);
    assert!(
        !base
            .with(Setting::ThirdPartyEmotes(false))
            .third_party_emotes
    );
    let quality = base.with(Setting::StartQuality(QualityPreference::AudioOnly));
    assert_eq!(
        quality,
        Settings {
            start_quality: QualityPreference::AudioOnly,
            ..base
        }
    );
}

#[test]
fn third_party_emotes_off_asks_no_provider() {
    let on = Settings::default();
    assert_eq!(
        on.emote_providers(),
        [
            EmoteProvider::SevenTv,
            EmoteProvider::Bttv,
            EmoteProvider::Ffz
        ]
    );
    let off = on.with(Setting::ThirdPartyEmotes(false));
    assert!(off.emote_providers().is_empty());
}

#[test]
fn names_round_trip_and_unknown_names_are_none() {
    for theme in ThemeChoice::ALL {
        assert_eq!(ThemeChoice::from_name(theme.name()), Some(theme));
    }
    for quality in QualityPreference::ALL {
        assert_eq!(QualityPreference::from_name(quality.name()), Some(quality));
    }
    assert_eq!(ThemeChoice::from_name("Dark"), None);
    assert_eq!(QualityPreference::from_name("1440p"), None);
    assert_eq!(QualityPreference::P1080.name(), "1080p");
    assert_eq!(QualityPreference::AudioOnly.name(), "audio_only");
}
