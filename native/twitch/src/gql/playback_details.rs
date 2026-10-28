//! What is playing, beside how to play it: raw documents written by us, one for
//! a live channel and one for a VOD. A VOD's streamed date is its `createdAt`;
//! `publishedAt` is when it was made public, which for an upload can be long
//! after. The channel is the live user itself, or the VOD's `owner`; one Twitch
//! cannot fully describe (missing a field, or an id or login core rejects) is
//! left unknown rather than failing the details. A live stream's viewers and
//! start are its `viewersCount` and `createdAt`; a stream without a viewer
//! count has no live details. A title Twitch sends empty is no title.

use jitter_core::models::{ChannelLogin, LiveDetails, PlaybackDetails, VideoId};
use jitter_core::playback::PlaybackTarget;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{GameNode, RawQuery, UserNode, parse_timestamp};
use crate::client::Twitch;
use crate::error::TwitchError;

pub const LIVE_QUERY: RawQuery = RawQuery {
    operation: "PlaybackDetailsLive",
    document: "query($login: String!) { user(login: $login) { \
               id login displayName profileImageURL(width: 50) \
               stream { title viewersCount createdAt game { id displayName } } } }",
};

pub const VOD_QUERY: RawQuery = RawQuery {
    operation: "PlaybackDetailsVod",
    document: "query($id: ID!) { video(id: $id) { title createdAt game { id displayName } \
               owner { id login displayName profileImageURL(width: 50) } } }",
};

pub fn live_variables(login: &ChannelLogin) -> Value {
    json!({ "login": login.as_str() })
}

pub fn vod_variables(id: &VideoId) -> Value {
    json!({ "id": id.as_str() })
}

/// Decodes a live response. An unknown login (`"user": null`) is
/// `NotFound`; an offline channel (`"stream": null`) has no game and no
/// live details.
pub fn parse_live(body: &[u8]) -> Result<PlaybackDetails, TwitchError> {
    map_live(super::decode(body, LIVE_QUERY.operation)?)
}

/// Decodes a VOD response. An unknown VOD (`"video": null`) is `NotFound`;
/// a missing or unreadable `createdAt` leaves the date unknown.
pub fn parse_vod(body: &[u8]) -> Result<PlaybackDetails, TwitchError> {
    map_vod(super::decode(body, VOD_QUERY.operation)?)
}

fn map_live(data: LiveData) -> Result<PlaybackDetails, TwitchError> {
    let user = data.user.ok_or(TwitchError::NotFound)?;
    let stream = user.stream;
    let live = stream.as_ref().and_then(Stream::live_details);
    Ok(PlaybackDetails {
        channel: user.node.into_channel(),
        title: stream.as_ref().and_then(|s| set_title(s.title.as_deref())),
        game: stream
            .and_then(|s| s.game)
            .and_then(GameNode::into_game_ref),
        streamed_at: None,
        live,
    })
}

fn map_vod(data: VodData) -> Result<PlaybackDetails, TwitchError> {
    let video = data.video.ok_or(TwitchError::NotFound)?;
    Ok(PlaybackDetails {
        channel: video.owner.and_then(UserNode::into_channel),
        title: set_title(video.title.as_deref()),
        game: video.game.and_then(GameNode::into_game_ref),
        streamed_at: video.created_at.as_deref().and_then(parse_timestamp),
        live: None,
    })
}

/// [title] unless Twitch sent it empty, as it does for a title never set.
fn set_title(title: Option<&str>) -> Option<String> {
    title
        .filter(|title| !title.trim().is_empty())
        .map(str::to_owned)
}

impl Twitch {
    /// What `target` is playing: the channel, and the game a live stream
    /// is in now, or the one a VOD was recorded in and when it was
    /// streamed.
    pub async fn playback_details(
        &self,
        target: &PlaybackTarget,
    ) -> Result<PlaybackDetails, TwitchError> {
        match target {
            PlaybackTarget::Live(login) => {
                map_live(self.query(&LIVE_QUERY, live_variables(login)).await?)
            }
            PlaybackTarget::Vod(id) => map_vod(self.query(&VOD_QUERY, vod_variables(id)).await?),
        }
    }
}

#[derive(Deserialize)]
struct LiveData {
    user: Option<User>,
}

#[derive(Deserialize)]
struct User {
    #[serde(flatten)]
    node: UserNode,
    stream: Option<Stream>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Stream {
    title: Option<String>,
    viewers_count: Option<u32>,
    created_at: Option<String>,
    game: Option<GameNode>,
}

impl Stream {
    fn live_details(&self) -> Option<LiveDetails> {
        Some(LiveDetails {
            viewers: self.viewers_count?,
            started_at: self.created_at.as_deref().and_then(parse_timestamp),
        })
    }
}

#[derive(Deserialize)]
struct VodData {
    video: Option<Video>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Video {
    title: Option<String>,
    created_at: Option<String>,
    game: Option<GameNode>,
    owner: Option<UserNode>,
}
