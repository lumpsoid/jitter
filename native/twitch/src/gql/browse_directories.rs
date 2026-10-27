//! Top categories: the web client's persisted `BrowsePage_AllDirectories`.

use jitter_core::models::{Category, CategoryId, Cursor, Page};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{PAGE_SIZE, PersistedQuery};
use crate::client::Twitch;
use crate::error::TwitchError;

pub const QUERY: PersistedQuery = PersistedQuery {
    operation: "BrowsePage_AllDirectories",
    hash: "2f67f71ba89f3c0ed26a141ec00da1defecb2303595f5cda4298169549783d9e",
    needs_device_id: false,
};

/// The variable set the persisted query expects. Sort `VIEWER_COUNT` and
/// no tag filter.
pub fn variables(cursor: Option<&Cursor>) -> Value {
    json!({
        "cursor": cursor.map(Cursor::as_str),
        "limit": PAGE_SIZE,
        "options": {
            "sort": "VIEWER_COUNT",
            "tags": []
        }
    })
}

/// Decodes a response body. Nodes missing an id, a name, box art or a
/// viewer count are dropped.
pub fn parse(body: &[u8]) -> Result<Page<Category>, TwitchError> {
    map(super::decode(body, QUERY.operation)?)
}

fn map(data: Data) -> Result<Page<Category>, TwitchError> {
    let directories = data.directories_with_tags.ok_or_else(|| {
        TwitchError::Decode("BrowsePage_AllDirectories: directoriesWithTags is null".into())
    })?;
    let next = next_cursor(&directories);
    let items = directories
        .edges
        .into_iter()
        .filter_map(|edge| edge.node?.into_category())
        .collect();
    Ok(Page { items, next })
}

impl Twitch {
    /// One page of categories by live viewers.
    pub async fn top_categories(
        &self,
        cursor: Option<&Cursor>,
    ) -> Result<Page<Category>, TwitchError> {
        map(self.persisted(&QUERY, variables(cursor)).await?)
    }
}

/// The last edge's cursor unless Twitch says this is the last page. A
/// missing `pageInfo` means more; a missing cursor ends the list either
/// way.
fn next_cursor(directories: &Directories) -> Option<Cursor> {
    let has_next = directories.page_info.as_ref().and_then(|p| p.has_next_page);
    if has_next == Some(false) {
        return None;
    }
    directories.edges.last()?.cursor.clone().map(Cursor::new)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Data {
    directories_with_tags: Option<Directories>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Directories {
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

/// `avatarURL` is the box art.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Node {
    id: Option<String>,
    display_name: Option<String>,
    #[serde(rename = "avatarURL")]
    avatar_url: Option<String>,
    viewers_count: Option<u32>,
}

impl Node {
    fn into_category(self) -> Option<Category> {
        Some(Category {
            id: CategoryId::parse(&self.id?).ok()?,
            name: self.display_name?,
            box_art_url: self.avatar_url?,
            viewers: self.viewers_count?,
        })
    }
}
