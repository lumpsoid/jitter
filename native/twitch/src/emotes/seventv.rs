//! 7TV. Zero-width is bit 0 of the active emote's top-level `flags` (not the
//! emote's own `data.flags`, which is a different field). Testing
//! `flags == 1` would miss an emote with other bits set, so jitter tests the
//! bit.

use jitter_core::models::{ChannelId, Emote};
use serde::Deserialize;

use super::absolute;
use crate::client::Twitch;
use crate::error::TwitchError;

/// `https://7tv.io/v3/emote-sets/global`, `…/users/twitch/{id}`.
const GLOBAL_URL: &str = "https://7tv.io/v3/emote-sets/global";
const USER_URL: &str = "https://7tv.io/v3/users/twitch";

/// Bit 0 of `flags`: the emote overlays the previous one.
const FLAG_ZERO_WIDTH: u64 = 1;

/// The image size shown in chat; every emote offers `1x`..`4x` as WebP.
const IMAGE_FILE: &str = "2x.webp";

pub fn channel_url(channel: &ChannelId) -> String {
    format!("{USER_URL}/{channel}")
}

pub(crate) async fn global(twitch: &Twitch) -> Result<Vec<Emote>, TwitchError> {
    let (_, body) = twitch.third_party_get(GLOBAL_URL).await?;
    parse_global(&body)
}

/// A channel without a 7TV account answers 404: no emotes, not a failure.
pub(crate) async fn channel(
    twitch: &Twitch,
    channel: &ChannelId,
) -> Result<Vec<Emote>, TwitchError> {
    match twitch.third_party_get(&channel_url(channel)).await {
        Ok((_, body)) => parse_channel(&body),
        Err(TwitchError::Http { status: 404 }) => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

#[derive(Deserialize)]
struct EmoteSet {
    #[serde(default)]
    emotes: Vec<Active>,
}

#[derive(Deserialize)]
struct User {
    emote_set: Option<EmoteSet>,
}

#[derive(Deserialize)]
struct Active {
    id: Option<String>,
    name: Option<String>,
    #[serde(default)]
    flags: u64,
    data: Option<Data>,
}

#[derive(Deserialize)]
struct Data {
    host: Option<Host>,
}

#[derive(Deserialize)]
struct Host {
    url: Option<String>,
}

pub fn parse_global(body: &[u8]) -> Result<Vec<Emote>, TwitchError> {
    let set: EmoteSet = decode(body)?;
    Ok(map(set))
}

pub fn parse_channel(body: &[u8]) -> Result<Vec<Emote>, TwitchError> {
    let user: User = decode(body)?;
    Ok(user.emote_set.map(map).unwrap_or_default())
}

fn decode<T: for<'de> Deserialize<'de>>(body: &[u8]) -> Result<T, TwitchError> {
    serde_json::from_slice(body).map_err(|e| TwitchError::Decode(format!("7TV: {e}")))
}

/// Emotes missing an id, name or image host are dropped.
fn map(set: EmoteSet) -> Vec<Emote> {
    set.emotes
        .into_iter()
        .filter_map(|e| {
            let host = e.data?.host?.url?;
            Some(Emote {
                id: e.id?,
                name: e.name?,
                url: format!("{}/{IMAGE_FILE}", absolute(&host)),
                zero_width: e.flags & FLAG_ZERO_WIDTH != 0,
            })
        })
        .collect()
}
