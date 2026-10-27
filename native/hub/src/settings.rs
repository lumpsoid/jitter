//! The settings page: settings signals ↔ the settings controller. A
//! change takes effect where it belongs: `run.rs` hands the new settings
//! to the chat at once, and playback and chat read them on the next `Play`.

use jitter_core::models::{QualityPreference, Setting, Settings, ThemeChoice};
use jitter_core::settings::SettingsController;
use rinf::RustSignal;

use crate::signals::{
    AppTheme, ChangeSetting, SettingChange, SettingsUpdate, StartQuality, StartQualityChoice,
};

/// `SettingsReady`: the settings as loaded at boot or last changed.
pub fn ready(settings: &SettingsController) {
    send_snapshot(settings.state());
}

/// `ChangeSetting`: stores the change and sends the settings, or the
/// failure. Returns whether they changed.
pub fn change(settings: &mut SettingsController, signal: ChangeSetting) -> bool {
    match settings.set(Setting::from(signal.change)) {
        Ok(()) => {
            send_snapshot(settings.state());
            true
        }
        Err(e) => {
            SettingsUpdate::Failed {
                message: e.to_string(),
            }
            .send_signal_to_dart();
            false
        }
    }
}

fn send_snapshot(settings: &Settings) {
    SettingsUpdate::Snapshot {
        theme: settings.theme.into(),
        animated_emotes: settings.animated_emotes,
        auto_load_chat: settings.auto_load_chat,
        start_quality: settings.start_quality.into(),
        start_qualities: QualityPreference::ALL
            .into_iter()
            .map(|quality| StartQualityChoice {
                quality: quality.into(),
                label: label(quality).to_owned(),
            })
            .collect(),
        third_party_emotes: settings.third_party_emotes,
        background_audio: settings.background_audio,
    }
    .send_signal_to_dart();
}

/// How the page names a start quality: `Source`, `720p`, `Audio only`.
fn label(quality: QualityPreference) -> &'static str {
    match quality {
        QualityPreference::Source => "Source",
        QualityPreference::P1080 => "1080p",
        QualityPreference::P720 => "720p",
        QualityPreference::P480 => "480p",
        QualityPreference::P360 => "360p",
        QualityPreference::P160 => "160p",
        QualityPreference::AudioOnly => "Audio only",
    }
}

impl From<SettingChange> for Setting {
    fn from(change: SettingChange) -> Self {
        match change {
            SettingChange::Theme { theme } => Self::Theme(theme.into()),
            SettingChange::AnimatedEmotes { on } => Self::AnimatedEmotes(on),
            SettingChange::AutoLoadChat { on } => Self::AutoLoadChat(on),
            SettingChange::StartQuality { quality } => Self::StartQuality(quality.into()),
            SettingChange::ThirdPartyEmotes { on } => Self::ThirdPartyEmotes(on),
            SettingChange::BackgroundAudio { on } => Self::BackgroundAudio(on),
        }
    }
}

impl From<ThemeChoice> for AppTheme {
    fn from(theme: ThemeChoice) -> Self {
        match theme {
            ThemeChoice::System => Self::System,
            ThemeChoice::Light => Self::Light,
            ThemeChoice::Dark => Self::Dark,
        }
    }
}

impl From<AppTheme> for ThemeChoice {
    fn from(theme: AppTheme) -> Self {
        match theme {
            AppTheme::System => Self::System,
            AppTheme::Light => Self::Light,
            AppTheme::Dark => Self::Dark,
        }
    }
}

impl From<QualityPreference> for StartQuality {
    fn from(quality: QualityPreference) -> Self {
        match quality {
            QualityPreference::Source => Self::Source,
            QualityPreference::P1080 => Self::P1080,
            QualityPreference::P720 => Self::P720,
            QualityPreference::P480 => Self::P480,
            QualityPreference::P360 => Self::P360,
            QualityPreference::P160 => Self::P160,
            QualityPreference::AudioOnly => Self::AudioOnly,
        }
    }
}

impl From<StartQuality> for QualityPreference {
    fn from(quality: StartQuality) -> Self {
        match quality {
            StartQuality::Source => Self::Source,
            StartQuality::P1080 => Self::P1080,
            StartQuality::P720 => Self::P720,
            StartQuality::P480 => Self::P480,
            StartQuality::P360 => Self::P360,
            StartQuality::P160 => Self::P160,
            StartQuality::AudioOnly => Self::AudioOnly,
        }
    }
}
