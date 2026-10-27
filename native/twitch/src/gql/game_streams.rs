//! One category's live streams: a raw `GameStreams` document, looked up by
//! id. The persisted `DirectoryPage_Game` needs the category's slug, which
//! a stream does not carry.

use jitter_core::models::{CategoryId, Cursor, Page, Stream};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{GameNode, PAGE_SIZE, RawQuery, UserNode, parse_timestamp};
use crate::client::Twitch;
use crate::error::TwitchError;

pub const QUERY: RawQuery = RawQuery {
    operation: "GameStreams",
    document: "query($id: ID, $sort: StreamSort, $first: Int, $after: Cursor) { \
               game(id: $id) { id displayName \
               streams(first: $first, after: $after, options: { sort: $sort }) { \
               edges { cursor node { id title viewersCount createdAt previewImageURL \
               broadcaster { id login displayName profileImageURL(width: 50) } } } \
               pageInfo { hasNextPage } } } }",
};

/// Sort `VIEWER_COUNT`, our default for a category's streams.
pub fn variables(id: &CategoryId, cursor: Option<&Cursor>) -> Value {
    json!({
        "id": id.as_str(),
        "sort": "VIEWER_COUNT",
        "first": PAGE_SIZE,
        "after": cursor.map(Cursor::as_str),
    })
}

/// Decodes a response body. An unknown id comes back as a `null` game and
/// is `NotFound`. Each stream is placed in the game it was listed under;
/// edges missing required data are dropped.
pub fn parse(body: &[u8]) -> Result<Page<Stream>, TwitchError> {
    map(super::decode(body, QUERY.operation)?)
}

fn map(data: Data) -> Result<Page<Stream>, TwitchError> {
    let game = data.game.ok_or(TwitchError::NotFound)?;
    let streams = game
        .streams
        .ok_or_else(|| TwitchError::Decode("GameStreams: streams is null".into()))?;
    let next = next_cursor(&streams);
    let game_ref = game.game.into_game_ref();
    let items = streams
        .edges
        .into_iter()
        .filter_map(|edge| {
            let mut stream = edge.node?.into_stream()?;
            stream.game.clone_from(&game_ref);
            Some(stream)
        })
        .collect();
    Ok(Page { items, next })
}

impl Twitch {
    /// One page of category `id`'s live streams by viewer count.
    pub async fn category_streams(
        &self,
        id: &CategoryId,
        cursor: Option<&Cursor>,
    ) -> Result<Page<Stream>, TwitchError> {
        map(self.query(&QUERY, variables(id, cursor)).await?)
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
    game: Option<Game>,
}

#[derive(Deserialize)]
struct Game {
    #[serde(flatten)]
    game: GameNode,
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
    created_at: Option<String>,
    #[serde(rename = "previewImageURL")]
    preview_image_url: Option<String>,
    broadcaster: Option<UserNode>,
}

impl Node {
    /// The stream without its game; [`map`] places it.
    fn into_stream(self) -> Option<Stream> {
        Some(Stream {
            id: self.id?,
            channel: self.broadcaster?.into_channel()?,
            title: self.title?,
            game: None,
            viewers: self.viewers_count?,
            started_at: self.created_at.as_deref().and_then(parse_timestamp),
            thumbnail_url: self.preview_image_url?,
        })
    }
}
