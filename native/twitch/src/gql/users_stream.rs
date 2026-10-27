//! Live status for a list of channel ids: a raw document written by us (no
//! persisted query for it; accepted anonymously, Phase 2).

use jitter_core::models::{ChannelId, Stream};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{GameNode, RawQuery, UserNode, parse_timestamp};
use crate::client::Twitch;
use crate::error::TwitchError;

/// Ids per request: `users(ids:)` takes at most 100.
pub const LIVE_STATUS_BATCH: usize = 100;

pub const QUERY: RawQuery = RawQuery {
    operation: "UsersStream",
    document: "query($ids: [ID!]) { users(ids: $ids) { id login displayName \
               profileImageURL(width: 50) stream { id title viewersCount createdAt \
               previewImageURL game { id displayName } } } }",
};

/// The variables of each request needed for `ids`, at most
/// `LIVE_STATUS_BATCH` ids each.
pub fn batch_variables(ids: &[ChannelId]) -> Vec<Value> {
    ids.chunks(LIVE_STATUS_BATCH)
        .map(|chunk| json!({ "ids": chunk.iter().map(ChannelId::as_str).collect::<Vec<_>>() }))
        .collect()
}

/// Decodes one response body into the streams of the users that are live.
/// Unknown ids come back as `null` users and offline users have a `null`
/// stream; both yield nothing. A user counts as live when its stream has a
/// viewer count.
pub fn parse(body: &[u8]) -> Result<Vec<Stream>, TwitchError> {
    map(super::decode(body, QUERY.operation)?)
}

fn map(data: Data) -> Result<Vec<Stream>, TwitchError> {
    let users = data
        .users
        .ok_or_else(|| TwitchError::Decode("UsersStream: users is null".into()))?;
    Ok(users
        .into_iter()
        .flatten()
        .filter_map(User::into_stream)
        .collect())
}

impl Twitch {
    /// The live streams among `ids`, in batches of `LIVE_STATUS_BATCH`. Any
    /// failed batch fails the whole call.
    pub async fn live_streams(&self, ids: &[ChannelId]) -> Result<Vec<Stream>, TwitchError> {
        let mut streams = Vec::new();
        for variables in batch_variables(ids) {
            streams.extend(map(self.query(&QUERY, variables).await?)?);
        }
        Ok(streams)
    }
}

#[derive(Deserialize)]
struct Data {
    users: Option<Vec<Option<User>>>,
}

#[derive(Deserialize)]
struct User {
    #[serde(flatten)]
    channel: UserNode,
    stream: Option<StreamNode>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StreamNode {
    id: Option<String>,
    title: Option<String>,
    viewers_count: Option<u32>,
    created_at: Option<String>,
    #[serde(rename = "previewImageURL")]
    preview_image_url: Option<String>,
    game: Option<GameNode>,
}

impl User {
    fn into_stream(self) -> Option<Stream> {
        let stream = self.stream?;
        Some(Stream {
            id: stream.id?,
            channel: self.channel.into_channel()?,
            title: stream.title?,
            game: stream.game.and_then(GameNode::into_game_ref),
            viewers: stream.viewers_count?,
            started_at: stream.created_at.as_deref().and_then(parse_timestamp),
            thumbnail_url: stream.preview_image_url?,
        })
    }
}
