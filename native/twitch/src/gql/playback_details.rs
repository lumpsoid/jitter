//! What is playing, beside how to play it: raw documents written by us, one for
//! a live channel and one for a VOD.

use jitter_core::models::{ChannelLogin, PlaybackDetails, VideoId};
use jitter_core::playback::PlaybackTarget;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{GameNode, RawQuery};
use crate::client::Twitch;
use crate::error::TwitchError;

pub const LIVE_QUERY: RawQuery = RawQuery {
    operation: "PlaybackDetailsLive",
    document: "query($login: String!) { user(login: $login) { \
               stream { game { id displayName } } } }",
};

pub const VOD_QUERY: RawQuery = RawQuery {
    operation: "PlaybackDetailsVod",
    document: "query($id: ID!) { video(id: $id) { game { id displayName } } }",
};

pub fn live_variables(login: &ChannelLogin) -> Value {
    json!({ "login": login.as_str() })
}

pub fn vod_variables(id: &VideoId) -> Value {
    json!({ "id": id.as_str() })
}

/// Decodes a live response. An unknown login (`"user": null`) is
/// `NotFound`; an offline channel (`"stream": null`) has no game.
pub fn parse_live(body: &[u8]) -> Result<PlaybackDetails, TwitchError> {
    map_live(super::decode(body, LIVE_QUERY.operation)?)
}

/// Decodes a VOD response. An unknown VOD (`"video": null`) is `NotFound`.
pub fn parse_vod(body: &[u8]) -> Result<PlaybackDetails, TwitchError> {
    map_vod(super::decode(body, VOD_QUERY.operation)?)
}

fn map_live(data: LiveData) -> Result<PlaybackDetails, TwitchError> {
    let user = data.user.ok_or(TwitchError::NotFound)?;
    Ok(details(user.stream.and_then(|s| s.game)))
}

fn map_vod(data: VodData) -> Result<PlaybackDetails, TwitchError> {
    let video = data.video.ok_or(TwitchError::NotFound)?;
    Ok(details(video.game))
}

fn details(game: Option<GameNode>) -> PlaybackDetails {
    PlaybackDetails {
        game: game.and_then(GameNode::into_game_ref),
    }
}

impl Twitch {
    /// What `target` is playing: the game a live stream is in now, or the
    /// one a VOD was recorded in.
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
    stream: Option<Stream>,
}

#[derive(Deserialize)]
struct Stream {
    game: Option<GameNode>,
}

#[derive(Deserialize)]
struct VodData {
    video: Option<Video>,
}

#[derive(Deserialize)]
struct Video {
    game: Option<GameNode>,
}
