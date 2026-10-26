//! FrankerFaceZ. No zero-width rule: `modifier` emotes are drawn as normal
//! emotes.

use std::collections::HashMap;

use jitter_core::models::{ChannelId, Emote};
use serde::Deserialize;

use super::absolute;
use crate::client::Twitch;
use crate::error::TwitchError;

/// `https://api.frankerfacez.com/v1/set/global`, `…/v1/room/id/{id}`.
const GLOBAL_URL: &str = "https://api.frankerfacez.com/v1/set/global";
const ROOM_URL: &str = "https://api.frankerfacez.com/v1/room/id";

/// `urls` is keyed by scale; `2` is shown, `1` is the fallback.
const PREFERRED_SCALES: [&str; 2] = ["2", "1"];

pub fn channel_url(channel: &ChannelId) -> String {
    format!("{ROOM_URL}/{channel}")
}

pub(crate) async fn global(twitch: &Twitch) -> Result<Vec<Emote>, TwitchError> {
    let (_, body) = twitch.third_party_get(GLOBAL_URL).await?;
    parse_global(&body)
}

/// A channel without an FFZ room answers 404: no emotes, not a failure.
pub(crate) async fn channel(
    twitch: &Twitch,
    channel: &ChannelId,
) -> Result<Vec<Emote>, TwitchError> {
    match twitch.third_party_get(&channel_url(channel)).await {
        Ok((_, body)) => parse_room(&body),
        Err(TwitchError::Http { status: 404 }) => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

#[derive(Deserialize)]
struct Set {
    #[serde(default)]
    emoticons: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    id: Option<u64>,
    name: Option<String>,
    #[serde(default)]
    urls: HashMap<String, String>,
}

#[derive(Deserialize)]
struct Global {
    default_sets: Vec<u64>,
    sets: HashMap<String, Set>,
}

#[derive(Deserialize)]
struct Room {
    room: RoomInfo,
    sets: HashMap<String, Set>,
}

#[derive(Deserialize)]
struct RoomInfo {
    set: u64,
}

/// The global emotes are the sets listed in `default_sets`; the response
/// carries other sets too.
pub fn parse_global(body: &[u8]) -> Result<Vec<Emote>, TwitchError> {
    let global: Global = decode(body)?;
    let mut sets = global.sets;
    Ok(global
        .default_sets
        .iter()
        .filter_map(|id| sets.remove(&id.to_string()))
        .flat_map(map)
        .collect())
}

/// The channel's emotes are the set named by `room.set`.
pub fn parse_room(body: &[u8]) -> Result<Vec<Emote>, TwitchError> {
    let mut room: Room = decode(body)?;
    Ok(room
        .sets
        .remove(&room.room.set.to_string())
        .map(map)
        .unwrap_or_default())
}

fn decode<T: for<'de> Deserialize<'de>>(body: &[u8]) -> Result<T, TwitchError> {
    serde_json::from_slice(body).map_err(|e| TwitchError::Decode(format!("FFZ: {e}")))
}

/// Entries missing an id, name or image are dropped.
fn map(set: Set) -> Vec<Emote> {
    set.emoticons
        .into_iter()
        .filter_map(|e| {
            let url = PREFERRED_SCALES
                .iter()
                .find_map(|scale| e.urls.get(*scale))?;
            Some(Emote {
                id: e.id?.to_string(),
                name: e.name?,
                url: absolute(url),
                zero_width: false,
            })
        })
        .collect()
}
