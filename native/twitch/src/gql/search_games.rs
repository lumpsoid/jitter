//! Category search: a raw `SearchGames` document. Only the fields a
//! [`Category`] carries are asked for.

use jitter_core::models::{Category, CategoryId, Cursor, Page, SearchQuery};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{PAGE_SIZE, RawQuery};
use crate::client::Twitch;
use crate::error::TwitchError;

pub const QUERY: RawQuery = RawQuery {
    operation: "SearchGames",
    document: "query($query: String!, $first: Int, $after: Cursor) { \
               searchCategories(query: $query, first: $first, after: $after) { \
               edges { cursor node { id displayName boxArtURL viewersCount } } \
               pageInfo { hasNextPage } } }",
};

pub fn variables(query: &SearchQuery, cursor: Option<&Cursor>) -> Value {
    json!({
        "query": query.as_str(),
        "first": PAGE_SIZE,
        "after": cursor.map(Cursor::as_str),
    })
}

/// Decodes a response body. Nodes missing an id, a name or box art are
/// dropped.
pub fn parse(body: &[u8]) -> Result<Page<Category>, TwitchError> {
    map(super::decode(body, QUERY.operation)?)
}

fn map(data: Data) -> Result<Page<Category>, TwitchError> {
    let found = data
        .search_categories
        .ok_or_else(|| TwitchError::Decode("SearchGames: searchCategories is null".into()))?;
    let next = next_cursor(&found);
    let items = found
        .edges
        .into_iter()
        .filter_map(|edge| edge.node?.into_category())
        .collect();
    Ok(Page { items, next })
}

impl Twitch {
    /// One page of the categories matching `query`, in Twitch's order.
    pub async fn search_categories(
        &self,
        query: &SearchQuery,
        cursor: Option<&Cursor>,
    ) -> Result<Page<Category>, TwitchError> {
        map(self.query(&QUERY, variables(query, cursor)).await?)
    }
}

/// The last edge's cursor unless Twitch says this is the last page. A
/// missing `pageInfo` means more; a missing cursor ends the list either
/// way.
fn next_cursor(found: &Found) -> Option<Cursor> {
    let has_next = found.page_info.as_ref().and_then(|p| p.has_next_page);
    if has_next == Some(false) {
        return None;
    }
    found.edges.last()?.cursor.clone().map(Cursor::new)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Data {
    search_categories: Option<Found>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Found {
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
    display_name: Option<String>,
    #[serde(rename = "boxArtURL")]
    box_art_url: Option<String>,
    viewers_count: Option<u32>,
}

impl Node {
    /// A category nobody is streaming has no viewer count: zero viewers.
    fn into_category(self) -> Option<Category> {
        Some(Category {
            id: CategoryId::parse(&self.id?).ok()?,
            name: self.display_name?,
            box_art_url: self.box_art_url?,
            viewers: self.viewers_count.unwrap_or(0),
        })
    }
}
