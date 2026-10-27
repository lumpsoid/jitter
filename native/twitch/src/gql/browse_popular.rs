//! Top live streams: the web client's persisted `BrowsePage_Popular`.

use jitter_core::models::{Cursor, Page, Stream};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{GameNode, PAGE_SIZE, PersistedQuery, UserNode};
use crate::client::Twitch;
use crate::error::TwitchError;

pub const QUERY: PersistedQuery = PersistedQuery {
    operation: "BrowsePage_Popular",
    hash: "fb60a7f9b2fe8f9c9a080f41585bd4564bea9d3030f4d7cb8ab7f9e99b1cee67",
    needs_device_id: false,
};

/// The full variable set the persisted query expects; a different shape
/// fails or returns nothing. Sort `VIEWER_COUNT` is the web default, so
/// `sortTypeIsRecency` is false.
pub fn variables(cursor: Option<&Cursor>) -> Value {
    json!({
        "cursor": cursor.map(Cursor::as_str),
        "imageWidth": 50,
        "includeCostreaming": true,
        "limit": PAGE_SIZE,
        "options": {
            "broadcasterLanguages": [],
            "freeformTags": [],
            "sort": "VIEWER_COUNT"
        },
        "platformType": "all",
        "sortTypeIsRecency": false
    })
}

/// Decodes a response body. Edges missing required data are dropped.
/// `started_at` is `None`: this operation carries no start time.
pub fn parse(body: &[u8]) -> Result<Page<Stream>, TwitchError> {
    map(super::decode(body, QUERY.operation)?)
}

fn map(data: Data) -> Result<Page<Stream>, TwitchError> {
    let streams = data
        .streams
        .ok_or_else(|| TwitchError::Decode("BrowsePage_Popular: streams is null".into()))?;
    let next = next_cursor(&streams);
    let items = streams
        .edges
        .into_iter()
        .filter_map(|edge| edge.node?.into_stream())
        .collect();
    Ok(Page { items, next })
}

impl Twitch {
    /// One page of live streams by viewer count.
    pub async fn top_streams(&self, cursor: Option<&Cursor>) -> Result<Page<Stream>, TwitchError> {
        map(self.persisted(&QUERY, variables(cursor)).await?)
    }
}

/// The last edge's cursor when Twitch says there is another page. Taken
/// before any edge is dropped, so a dropped last edge does not refetch.
fn next_cursor(streams: &Streams) -> Option<Cursor> {
    if streams.page_info.as_ref()?.has_next_page != Some(true) {
        return None;
    }
    streams.edges.last()?.cursor.clone().map(Cursor::new)
}

#[derive(Deserialize)]
struct Data {
    streams: Option<Streams>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Streams {
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
    viewers_count: Option<u32>,
    #[serde(rename = "previewImageURL")]
    preview_image_url: Option<String>,
    broadcaster: Option<UserNode>,
    game: Option<GameNode>,
}

impl Node {
    fn into_stream(self) -> Option<Stream> {
        Some(Stream {
            id: self.id?,
            channel: self.broadcaster?.into_channel()?,
            title: self.title?,
            game: self.game.and_then(GameNode::into_game_ref),
            viewers: self.viewers_count?,
            started_at: None,
            thumbnail_url: self.preview_image_url?,
        })
    }
}
