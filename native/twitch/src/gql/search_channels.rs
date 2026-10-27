//! Channel search: a raw `SearchChannels` document.

use jitter_core::models::{Cursor, FoundChannel, Page, SearchQuery};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{PAGE_SIZE, RawQuery, UserNode};
use crate::client::Twitch;
use crate::error::TwitchError;

pub const QUERY: RawQuery = RawQuery {
    operation: "SearchChannels",
    document: "query($query: String!, $first: Int, $after: Cursor) { \
               searchUsers(userQuery: $query, first: $first, after: $after) { \
               edges { cursor node { id login displayName profileImageURL(width: 300) \
               followers { totalCount } stream { viewersCount } } } \
               pageInfo { hasNextPage } } }",
};

pub fn variables(query: &SearchQuery, cursor: Option<&Cursor>) -> Value {
    json!({
        "query": query.as_str(),
        "first": PAGE_SIZE,
        "after": cursor.map(Cursor::as_str),
    })
}

/// Decodes a response body. Users missing an id, a login, a name or an
/// avatar are dropped; a user without a stream is offline.
pub fn parse(body: &[u8]) -> Result<Page<FoundChannel>, TwitchError> {
    map(super::decode(body, QUERY.operation)?)
}

fn map(data: Data) -> Result<Page<FoundChannel>, TwitchError> {
    let users = data
        .search_users
        .ok_or_else(|| TwitchError::Decode("SearchChannels: searchUsers is null".into()))?;
    let next = next_cursor(&users);
    let items = users
        .edges
        .into_iter()
        .filter_map(|edge| edge.node?.into_found())
        .collect();
    Ok(Page { items, next })
}

impl Twitch {
    /// One page of the channels matching `query`, in Twitch's order.
    pub async fn search_channels(
        &self,
        query: &SearchQuery,
        cursor: Option<&Cursor>,
    ) -> Result<Page<FoundChannel>, TwitchError> {
        map(self.query(&QUERY, variables(query, cursor)).await?)
    }
}

/// The last edge's cursor unless Twitch says this is the last page. A
/// missing `pageInfo` means more; a missing cursor ends the list either
/// way.
fn next_cursor(users: &Users) -> Option<Cursor> {
    let has_next = users.page_info.as_ref().and_then(|p| p.has_next_page);
    if has_next == Some(false) {
        return None;
    }
    users.edges.last()?.cursor.clone().map(Cursor::new)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Data {
    search_users: Option<Users>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Users {
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
struct Node {
    #[serde(flatten)]
    user: UserNode,
    followers: Option<Followers>,
    stream: Option<LiveStream>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Followers {
    total_count: Option<u32>,
}

/// A user with a stream is live.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LiveStream {
    viewers_count: Option<u32>,
}

impl Node {
    fn into_found(self) -> Option<FoundChannel> {
        Some(FoundChannel {
            channel: self.user.into_channel()?,
            followers: self.followers.and_then(|f| f.total_count),
            viewers: self.stream.and_then(|s| s.viewers_count),
        })
    }
}
