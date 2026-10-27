//! Twitch GQL: request bodies, response classification, and one module per
//! operation. Every response is checked for `errors[]` before `data` is
//! read, so an error body is never mistaken for an empty result.

pub mod browse_directories;
pub mod browse_popular;
pub mod channel_by_login;
pub mod channel_videos;
pub mod game_streams;
pub mod playback_access_token;
pub mod search_channels;
pub mod search_games;
pub mod users_stream;

use jitter_core::models::{CategoryId, Channel, ChannelId, ChannelLogin, GameRef, Timestamp};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::client::Twitch;
use crate::error::TwitchError;

/// Items requested per page for paged operations. jitter's choice; the
/// Phase 2 fixtures were recorded with it.
pub const PAGE_SIZE: u32 = 30;

/// The error message Twitch sends when it wants an integrity token.
const FAILED_INTEGRITY_CHECK: &str = "failed integrity check";

/// `errors[].extensions.code` for an unknown persisted-query hash (observed:
/// fixture `gql_persisted_bogus_hash.json`).
const PERSISTED_QUERY_NOT_FOUND: &str = "PERSISTED_QUERY_NOT_FOUND";

/// A persisted operation the web client sends: its name, its hash, and
/// whether Twitch expects an `X-Device-Id` with it.
#[derive(Debug, Clone, Copy)]
pub struct PersistedQuery {
    pub operation: &'static str,
    pub hash: &'static str,
    pub needs_device_id: bool,
}

/// A raw query document written by us. `operation` only labels errors; the
/// body carries no operation name.
#[derive(Debug, Clone, Copy)]
pub struct RawQuery {
    pub operation: &'static str,
    pub document: &'static str,
}

/// `{"operationName","variables","extensions":{"persistedQuery":{…}}}`.
pub fn persisted_body(query: &PersistedQuery, variables: Value) -> Value {
    json!({
        "operationName": query.operation,
        "variables": variables,
        "extensions": {
            "persistedQuery": { "version": 1, "sha256Hash": query.hash }
        }
    })
}

/// `{"variables","query"}`.
pub fn query_body(query: &RawQuery, variables: Value) -> Value {
    json!({ "variables": variables, "query": query.document })
}

/// Decodes a GQL response body into `T` (the type of `data`). `errors[]` is
/// classified first; a body with any error is an `Err` even if it also
/// carries data. A body without errors and without `data` is `Decode`.
pub fn decode<T: DeserializeOwned>(body: &[u8], operation: &'static str) -> Result<T, TwitchError> {
    let errors: ErrorsOnly =
        serde_json::from_slice(body).map_err(|e| TwitchError::Decode(e.to_string()))?;
    if let Some(error) = classify(&errors.errors.unwrap_or_default(), operation) {
        return Err(error);
    }
    let data: DataOnly<T> =
        serde_json::from_slice(body).map_err(|e| TwitchError::Decode(e.to_string()))?;
    data.data
        .ok_or_else(|| TwitchError::Decode(format!("{operation}: no data and no errors")))
}

fn classify(errors: &[GqlError], operation: &'static str) -> Option<TwitchError> {
    if errors.is_empty() {
        return None;
    }
    if errors
        .iter()
        .any(|e| e.code() == Some(PERSISTED_QUERY_NOT_FOUND))
    {
        return Some(TwitchError::QueryRetired { operation });
    }
    if errors.iter().any(|e| e.message == FAILED_INTEGRITY_CHECK) {
        return Some(TwitchError::IntegrityRequired);
    }
    Some(TwitchError::Gql(
        errors.iter().map(|e| e.message.clone()).collect(),
    ))
}

/// RFC 3339 (as Twitch sends `createdAt`/`publishedAt`) to `Timestamp`.
pub(crate) fn parse_timestamp(raw: &str) -> Option<Timestamp> {
    OffsetDateTime::parse(raw, &Rfc3339)
        .ok()
        .map(|t| Timestamp::from_unix(t.unix_timestamp()))
}

/// The user fields every operation reads (`id login displayName
/// profileImageURL`). A user missing any of them, or with an id or login
/// core rejects, maps to `None` and its record is dropped.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserNode {
    id: Option<String>,
    login: Option<String>,
    display_name: Option<String>,
    #[serde(rename = "profileImageURL")]
    profile_image_url: Option<String>,
}

impl UserNode {
    pub(crate) fn into_channel(self) -> Option<Channel> {
        Some(Channel {
            id: ChannelId::parse(&self.id?).ok()?,
            login: ChannelLogin::parse(&self.login?).ok()?,
            display_name: self.display_name?,
            avatar_url: self.profile_image_url?,
        })
    }
}

/// A user's `followers { totalCount }`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Followers {
    pub(crate) total_count: Option<u32>,
}

/// The category fields a stream links by (`game { id displayName }`). A
/// game missing either, or with an id core rejects, maps to `None`: the
/// stream is kept, in no category.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GameNode {
    id: Option<String>,
    display_name: Option<String>,
}

impl GameNode {
    pub(crate) fn into_game_ref(self) -> Option<GameRef> {
        Some(GameRef {
            id: CategoryId::parse(&self.id?).ok()?,
            name: self.display_name?,
        })
    }
}

impl Twitch {
    pub(crate) async fn persisted<T: DeserializeOwned>(
        &self,
        query: &PersistedQuery,
        variables: Value,
    ) -> Result<T, TwitchError> {
        let body = self
            .post_gql(&persisted_body(query, variables), query.needs_device_id)
            .await?;
        decode(&body, query.operation)
    }

    pub(crate) async fn query<T: DeserializeOwned>(
        &self,
        query: &RawQuery,
        variables: Value,
    ) -> Result<T, TwitchError> {
        let body = self.post_gql(&query_body(query, variables), false).await?;
        decode(&body, query.operation)
    }
}

#[derive(Deserialize)]
struct ErrorsOnly {
    errors: Option<Vec<GqlError>>,
}

#[derive(Deserialize)]
struct DataOnly<T> {
    data: Option<T>,
}

#[derive(Deserialize)]
struct GqlError {
    #[serde(default)]
    message: String,
    #[serde(default)]
    extensions: Option<ErrorExtensions>,
}

impl GqlError {
    fn code(&self) -> Option<&str> {
        self.extensions.as_ref()?.code.as_deref()
    }
}

#[derive(Deserialize)]
struct ErrorExtensions {
    code: Option<String>,
}
