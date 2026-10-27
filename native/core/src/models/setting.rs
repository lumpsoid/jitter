use super::{QualityPreference, ThemeChoice};

/// One setting with its new value: what a user changes at a time, and what
/// a store writes at a time, so a head never overwrites a setting another
/// head changed meanwhile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    Theme(ThemeChoice),
    AnimatedEmotes(bool),
    AutoLoadChat(bool),
    StartQuality(QualityPreference),
    ThirdPartyEmotes(bool),
    BackgroundAudio(bool),
}
