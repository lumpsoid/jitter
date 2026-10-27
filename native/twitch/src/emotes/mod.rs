//! Third-party emote providers (7TV, BTTV, FFZ) and the Twitch emote image
//! URL. Fetches the provider lists and maps them to core `Emote`s; which
//! emote wins a name clash is core's `EmoteSet`.

pub mod bttv;
pub mod ffz;
pub mod seventv;

use futures_util::future::join_all;
use jitter_core::models::{ChannelId, Emote, EmoteProvider};

use crate::client::Twitch;
use crate::error::TwitchError;

/// Twitch's own emote image: `{id}/{format}/dark/{1.0|2.0|3.0}`. The
/// `default` format animates an animated emote; `static` is its first
/// frame, and the same image as `default` for an emote that never moves.
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

/// The still image of Twitch's emote `id`: its `static` format.
pub fn twitch_still_emote_url(id: &str, scale: TwitchEmoteScale) -> String {
    format!("{TWITCH_EMOTE_URL}/{id}/static/dark/{}", scale.as_str())
}

/// Twitch's emote `id`, written `name` in a message. Every Twitch emote
/// has a still, so animated or not it carries one.
pub(crate) fn twitch_emote(id: String, name: String) -> Emote {
    Emote {
        url: twitch_emote_url(&id, TwitchEmoteScale::Two),
        still_url: Some(twitch_still_emote_url(&id, TwitchEmoteScale::Two)),
        id,
        name,
        zero_width: false,
    }
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
    /// The global emote lists of `providers`, fetched concurrently, in
    /// their order. No provider is asked that is not listed.
    pub async fn global_emotes(&self, providers: &[EmoteProvider]) -> Vec<ProviderEmotes> {
        join_all(providers.iter().map(|&provider| async move {
            let result = match provider {
                EmoteProvider::SevenTv => seventv::global(self).await,
                EmoteProvider::Bttv => bttv::global(self).await,
                EmoteProvider::Ffz => ffz::global(self).await,
            };
            (provider, result)
        }))
        .await
    }

    /// One channel's emote lists from `providers`, fetched concurrently,
    /// in their order. A channel unknown to a provider has an empty list
    /// there. No provider is asked that is not listed.
    pub async fn channel_emotes(
        &self,
        channel: &ChannelId,
        providers: &[EmoteProvider],
    ) -> Vec<ProviderEmotes> {
        join_all(providers.iter().map(|&provider| async move {
            let result = match provider {
                EmoteProvider::SevenTv => seventv::channel(self, channel).await,
                EmoteProvider::Bttv => bttv::channel(self, channel).await,
                EmoteProvider::Ffz => ffz::channel(self, channel).await,
            };
            (provider, result)
        }))
        .await
    }
}
