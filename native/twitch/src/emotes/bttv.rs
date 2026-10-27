//! BTTV. The API has no zero-width flag, so we keep a fixed name list.

use jitter_core::models::{ChannelId, Emote};
use serde::Deserialize;

use crate::client::Twitch;
use crate::error::TwitchError;

/// `https://api.betterttv.net/3/cached/emotes/global`, `…/users/twitch/{id}`.
const GLOBAL_URL: &str = "https://api.betterttv.net/3/cached/emotes/global";
const USER_URL: &str = "https://api.betterttv.net/3/cached/users/twitch";

/// Emote images; `2x` is shown.
/// An animated emote's still is under `static/` (`{id}/static/2x.webp`),
/// as the BetterTTV extension's CDN helper builds it.
const IMAGE_URL: &str = "https://cdn.betterttv.net/emote";
const IMAGE_FILE: &str = "2x.webp";
const STILL_DIR: &str = "static";

/// Names BTTV draws over the previous emote.
const ZERO_WIDTH: [&str; 8] = [
    "IceCold",
    "SoSnowy",
    "SantaHat",
    "TopHat",
    "CandyCane",
    "ReinDeer",
    "cvHazmat",
    "cvMask",
];

pub fn channel_url(channel: &ChannelId) -> String {
    format!("{USER_URL}/{channel}")
}

pub(crate) async fn global(twitch: &Twitch) -> Result<Vec<Emote>, TwitchError> {
    let (_, body) = twitch.third_party_get(GLOBAL_URL).await?;
    parse_global(&body)
}

/// A channel without a BTTV account answers 404: no emotes, not a failure.
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
struct Entry {
    id: Option<String>,
    code: Option<String>,
    #[serde(default)]
    animated: bool,
}

#[derive(Deserialize)]
struct Channel {
    #[serde(rename = "channelEmotes", default)]
    channel_emotes: Vec<Entry>,
    #[serde(rename = "sharedEmotes", default)]
    shared_emotes: Vec<Entry>,
}

pub fn parse_global(body: &[u8]) -> Result<Vec<Emote>, TwitchError> {
    let entries: Vec<Entry> =
        serde_json::from_slice(body).map_err(|e| TwitchError::Decode(format!("BTTV: {e}")))?;
    Ok(map(entries))
}

pub fn parse_channel(body: &[u8]) -> Result<Vec<Emote>, TwitchError> {
    let channel: Channel =
        serde_json::from_slice(body).map_err(|e| TwitchError::Decode(format!("BTTV: {e}")))?;
    Ok(map(channel
        .channel_emotes
        .into_iter()
        .chain(channel.shared_emotes)))
}

/// Entries missing an id or code are dropped.
fn map(entries: impl IntoIterator<Item = Entry>) -> Vec<Emote> {
    entries
        .into_iter()
        .filter_map(|e| {
            let (id, name) = (e.id?, e.code?);
            Some(Emote {
                url: format!("{IMAGE_URL}/{id}/{IMAGE_FILE}"),
                still_url: e
                    .animated
                    .then(|| format!("{IMAGE_URL}/{id}/{STILL_DIR}/{IMAGE_FILE}")),
                zero_width: ZERO_WIDTH.contains(&name.as_str()),
                id,
                name,
            })
        })
        .collect()
}
