//! Third-party emote providers (7TV, BTTV, FFZ) and the Twitch emote image
//! URL. Fetches the provider lists and maps them to core `Emote`s; which
//! emote wins a name clash is core's `EmoteSet`.

pub mod bttv;
pub mod ffz;
pub mod seventv;

use jitter_core::models::{ChannelId, Emote, EmoteProvider};

use crate::client::Twitch;
use crate::error::TwitchError;

/// Twitch's own emote image: `{id}/default/dark/{1.0|2.0|3.0}`.
const TWITCH_EMOTE_URL: &str = "https://static-cdn.jtvnw.net/emoticons/v2";

/// The image scales Twitch offers for its emotes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwitchEmoteScale {
    One,
    Two,
    Three,
}

impl TwitchEmoteScale {
    fn as_str(self) -> &'static str {
        match self {
            Self::One => "1.0",
            Self::Two => "2.0",
            Self::Three => "3.0",
        }
    }
}

pub fn twitch_emote_url(id: &str, scale: TwitchEmoteScale) -> String {
    format!("{TWITCH_EMOTE_URL}/{id}/default/dark/{}", scale.as_str())
}

/// One provider's outcome. A failed provider is reported beside the others
/// (core: `ChatController::provider_failed`); it never fails the set.
pub type ProviderEmotes = (EmoteProvider, Result<Vec<Emote>, TwitchError>);

/// Makes a protocol-relative URL (`//cdn...`) absolute; other URLs are kept.
fn absolute(url: &str) -> String {
    match url.strip_prefix("//") {
        Some(rest) => format!("https://{rest}"),
        None => url.to_owned(),
    }
}

impl Twitch {
    /// The global emote lists of all three providers, fetched concurrently.
    pub async fn global_emotes(&self) -> [ProviderEmotes; 3] {
        let (seventv, bttv, ffz) =
            tokio::join!(seventv::global(self), bttv::global(self), ffz::global(self));
        [
            (EmoteProvider::SevenTv, seventv),
            (EmoteProvider::Bttv, bttv),
            (EmoteProvider::Ffz, ffz),
        ]
    }

    /// One channel's emote lists from all three providers, fetched
    /// concurrently. A channel unknown to a provider has an empty list there.
    pub async fn channel_emotes(&self, channel: &ChannelId) -> [ProviderEmotes; 3] {
        let (seventv, bttv, ffz) = tokio::join!(
            seventv::channel(self, channel),
            bttv::channel(self, channel),
            ffz::channel(self, channel)
        );
        [
            (EmoteProvider::SevenTv, seventv),
            (EmoteProvider::Bttv, bttv),
            (EmoteProvider::Ffz, ffz),
        ]
    }
}
