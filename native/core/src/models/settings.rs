use super::{EmoteProvider, QualityPreference, Setting, ThemeChoice};

/// The user's settings. Each takes effect where it belongs; heads pass
/// them to the controllers that need them as plain values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    pub theme: ThemeChoice,
    /// Animated emotes move; off, chat shows their still image where the
    /// provider offers one ([`super::Emote::image_url`]).
    pub animated_emotes: bool,
    /// Opening a player page joins its chat; off, the chat starts hidden
    /// until the user shows it.
    pub auto_load_chat: bool,
    /// The quality a player page opens at. Switching quality on a page
    /// does not change it.
    pub start_quality: QualityPreference,
    /// 7TV, BTTV and FFZ emotes are loaded; off, only Twitch's own render.
    pub third_party_emotes: bool,
    /// Leaving the app keeps the player going on the audio-only quality;
    /// off, it stops until the app is back in front.
    pub background_audio: bool,
}

/// What a user who never changed anything gets: the system theme, the
/// source quality, and everything else on.
impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::System,
            animated_emotes: true,
            auto_load_chat: true,
            start_quality: QualityPreference::Source,
            third_party_emotes: true,
            background_audio: true,
        }
    }
}

impl Settings {
    /// These settings with `setting` changed.
    pub fn with(self, setting: Setting) -> Self {
        match setting {
            Setting::Theme(theme) => Self { theme, ..self },
            Setting::AnimatedEmotes(animated_emotes) => Self {
                animated_emotes,
                ..self
            },
            Setting::AutoLoadChat(auto_load_chat) => Self {
                auto_load_chat,
                ..self
            },
            Setting::StartQuality(start_quality) => Self {
                start_quality,
                ..self
            },
            Setting::ThirdPartyEmotes(third_party_emotes) => Self {
                third_party_emotes,
                ..self
            },
            Setting::BackgroundAudio(background_audio) => Self {
                background_audio,
                ..self
            },
        }
    }

    /// The third-party providers a chat asks for emotes: all of them, or
    /// none while [`Self::third_party_emotes`] is off.
    pub fn emote_providers(&self) -> &'static [EmoteProvider] {
        match self.third_party_emotes {
            true => &EmoteProvider::ALL,
            false => &[],
        }
    }
}
