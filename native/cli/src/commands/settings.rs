use clap::{Subcommand, ValueEnum};
use jitter_core::models::{QualityPreference, Setting, Settings, ThemeChoice};
use jitter_core::settings::SettingsController;

use super::{Result, ctx, open_store};

#[derive(Subcommand)]
pub enum SettingsCommand {
    /// Change one setting, then show them all. Values: `theme` system,
    /// light or dark; `start-quality` source, 1080p, 720p, 480p, 360p, 160p
    /// or audio_only; the others on or off.
    Set { key: SettingKey, value: String },
}

/// The settings by the names the CLI shows and takes.
#[derive(Clone, Copy, ValueEnum)]
pub enum SettingKey {
    Theme,
    AnimatedEmotes,
    AutoLoadChat,
    StartQuality,
    ThirdPartyEmotes,
}

pub fn run(command: Option<SettingsCommand>) -> Result<()> {
    let mut settings = SettingsController::new(Box::new(open_store()?));
    settings.load().map_err(ctx("loading the settings"))?;
    if let Some(SettingsCommand::Set { key, value }) = command {
        settings
            .set(setting(key, &value)?)
            .map_err(ctx("saving the setting"))?;
    }
    print(settings.state());
    Ok(())
}

/// `key` set to `value`, parsed by core's names (`dark`, `720p`) or as
/// `on`/`off`.
fn setting(key: SettingKey, value: &str) -> Result<Setting> {
    let parsed = match key {
        SettingKey::Theme => ThemeChoice::from_name(value).map(Setting::Theme),
        SettingKey::StartQuality => QualityPreference::from_name(value).map(Setting::StartQuality),
        SettingKey::AnimatedEmotes => on_off(value).map(Setting::AnimatedEmotes),
        SettingKey::AutoLoadChat => on_off(value).map(Setting::AutoLoadChat),
        SettingKey::ThirdPartyEmotes => on_off(value).map(Setting::ThirdPartyEmotes),
    };
    parsed.ok_or_else(|| format!("invalid value {value:?}: expected {}", expected(key)))
}

/// The values `key` takes: `system, light, dark`.
fn expected(key: SettingKey) -> String {
    match key {
        SettingKey::Theme => ThemeChoice::ALL.map(ThemeChoice::name).join(", "),
        SettingKey::StartQuality => QualityPreference::ALL
            .map(QualityPreference::name)
            .join(", "),
        SettingKey::AnimatedEmotes | SettingKey::AutoLoadChat | SettingKey::ThirdPartyEmotes => {
            "on, off".to_owned()
        }
    }
}

fn on_off(value: &str) -> Option<bool> {
    match value {
        "on" => Some(true),
        "off" => Some(false),
        _ => None,
    }
}

fn shown(on: bool) -> &'static str {
    match on {
        true => "on",
        false => "off",
    }
}

fn print(settings: &Settings) {
    let rows = [
        ("theme", settings.theme.name()),
        ("animated-emotes", shown(settings.animated_emotes)),
        ("auto-load-chat", shown(settings.auto_load_chat)),
        ("start-quality", settings.start_quality.name()),
        ("third-party-emotes", shown(settings.third_party_emotes)),
    ];
    for (key, value) in rows {
        println!("{key:<20} {value}");
    }
}
