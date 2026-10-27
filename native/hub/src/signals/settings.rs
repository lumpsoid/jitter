use rinf::{DartSignal, RustSignal, SignalPiece};
use serde::{Deserialize, Serialize};

/// The settings holder is subscribed: answer with a `Snapshot`.
#[derive(Deserialize, DartSignal)]
pub struct SettingsReady;

/// The user changed one setting. Rust stores it and answers with a
/// `Snapshot`, or with `Failed` when it could not be stored.
#[derive(Deserialize, DartSignal)]
pub struct ChangeSetting {
    pub change: SettingChange,
}

/// One setting with its new value.
#[derive(Deserialize, SignalPiece)]
pub enum SettingChange {
    Theme { theme: AppTheme },
    AnimatedEmotes { on: bool },
    AutoLoadChat { on: bool },
    StartQuality { quality: StartQuality },
    ThirdPartyEmotes { on: bool },
    BackgroundAudio { on: bool },
}

/// The settings: a whole snapshot after every change. One type, so a
/// snapshot and the failures around it arrive in order.
#[derive(Serialize, RustSignal)]
pub enum SettingsUpdate {
    /// Every setting, and the qualities the page offers. Replaces them.
    Snapshot {
        theme: AppTheme,
        animated_emotes: bool,
        auto_load_chat: bool,
        start_quality: StartQuality,
        /// Every start quality, best first, to choose from.
        start_qualities: Vec<StartQualityChoice>,
        third_party_emotes: bool,
        background_audio: bool,
    },
    /// A change could not be stored; the settings are as last sent. A
    /// one-shot notice for a snackbar, not state.
    Failed { message: String },
}

/// The app's brightness: the platform's, or fixed.
#[derive(Serialize, Deserialize, SignalPiece)]
pub enum AppTheme {
    System,
    Light,
    Dark,
}

/// The quality a player page opens at; the closest lower one when the
/// stream lacks it.
#[derive(Serialize, Deserialize, SignalPiece)]
pub enum StartQuality {
    Source,
    P1080,
    P720,
    P480,
    P360,
    P160,
    AudioOnly,
}

/// One start quality the page offers: `label` is shown (`720p`).
#[derive(Serialize, SignalPiece)]
pub struct StartQualityChoice {
    pub quality: StartQuality,
    pub label: String,
}
