//! A channel's VODs: the web client's persisted `FilterableVideoTower_Videos`.

use jitter_core::models::{ChannelLogin, Cursor, Page, Seconds, Video, VideoId, VideoKind};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{GameNode, PAGE_SIZE, PersistedQuery, UserNode, parse_timestamp};
use crate::client::Twitch;
use crate::error::TwitchError;

pub const QUERY: PersistedQuery = PersistedQuery {
    operation: "FilterableVideoTower_Videos",
    hash: "53dc989e1d3cc360f7e4868c88eaaf37e06495b1d58c91c435e9e83643bb8c5c",
    needs_device_id: false,
};

/// Variable values `ARCHIVE`/`HIGHLIGHT`/`UPLOAD` and sort `TIME`.
pub fn variables(login: &ChannelLogin, kind: VideoKind, cursor: Option<&Cursor>) -> Value {
    json!({
        "broadcastType": broadcast_type(kind),
        "channelOwnerLogin": login.as_str(),
        "cursor": cursor.map(Cursor::as_str),
        "includePreviewBlur": false,
        "limit": PAGE_SIZE,
        "videoSort": "TIME"
    })
}

fn broadcast_type(kind: VideoKind) -> &'static str {
    match kind {
        VideoKind::Archive => "ARCHIVE",
        VideoKind::Highlight => "HIGHLIGHT",
        VideoKind::Upload => "UPLOAD",
    }
}

/// Decodes a response body for a request of `kind`. The nodes carry no
/// broadcast type, so every video gets the requested `kind`. Edges missing
/// required data are dropped; an unknown login (`"user": null`) is
/// `NotFound`.
pub fn parse(body: &[u8], kind: VideoKind) -> Result<Page<Video>, TwitchError> {
    map(super::decode(body, QUERY.operation)?, kind)
}

fn map(data: Data, kind: VideoKind) -> Result<Page<Video>, TwitchError> {
    let videos = data
        .user
        .ok_or(TwitchError::NotFound)?
        .videos
        .ok_or_else(|| TwitchError::Decode("FilterableVideoTower_Videos: videos is null".into()))?;
    let next = next_cursor(&videos);
    let items = videos
        .edges
        .into_iter()
        .filter_map(|edge| edge.node?.into_video(kind))
        .collect();
    Ok(Page { items, next })
}

impl Twitch {
    /// One page of a channel's videos of one kind, newest first.
    pub async fn channel_videos(
        &self,
        login: &ChannelLogin,
        kind: VideoKind,
        cursor: Option<&Cursor>,
    ) -> Result<Page<Video>, TwitchError> {
        map(
            self.persisted(&QUERY, variables(login, kind, cursor))
                .await?,
            kind,
        )
    }
}

/// The last edge's cursor when Twitch says there is another page, taken
/// before any edge is dropped.
fn next_cursor(videos: &Videos) -> Option<Cursor> {
    if videos.page_info.as_ref()?.has_next_page != Some(true) {
        return None;
    }
    videos.edges.last()?.cursor.clone().map(Cursor::new)
}

#[derive(Deserialize)]
struct Data {
    user: Option<User>,
}

#[derive(Deserialize)]
struct User {
    videos: Option<Videos>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Videos {
    #[serde(default)]
    edges: Vec<Edge>,
    page_info: Option<PageInfo>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    has_next_page: Option<bool>,
}

#[derive(Deserialize)]
struct Edge {
    cursor: Option<String>,
    node: Option<Node>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Node {
    id: Option<String>,
    title: Option<String>,
    length_seconds: Option<u32>,
    published_at: Option<String>,
    #[serde(rename = "previewThumbnailURL")]
    preview_thumbnail_url: Option<String>,
    owner: Option<UserNode>,
    game: Option<GameNode>,
}

impl Node {
    fn into_video(self, kind: VideoKind) -> Option<Video> {
        Some(Video {
            id: VideoId::parse(&self.id?).ok()?,
            channel: self.owner?.into_channel()?,
            title: self.title?,
            duration: Seconds::new(self.length_seconds?),
            published_at: parse_timestamp(&self.published_at?)?,
            thumbnail_url: self.preview_thumbnail_url?,
            kind,
            game: self.game.and_then(GameNode::into_game_ref),
        })
    }
}
