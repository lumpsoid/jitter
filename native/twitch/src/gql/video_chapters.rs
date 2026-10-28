//! A VOD's chapters: its game-change moments from a raw `VideoMoments`
//! document, the first 100. A moment Twitch cannot fully describe (no
//! position, or no game core accepts) is dropped; the rest still are the
//! chapters. A position is in milliseconds, cut to the second.

use jitter_core::models::{Chapter, Chapters, Seconds, VideoId};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{GameNode, RawQuery};
use crate::client::Twitch;
use crate::error::TwitchError;

pub const QUERY: RawQuery = RawQuery {
    operation: "VideoChapters",
    document: "query($id: ID!) { video(id: $id) { \
               moments(first: 100, sort: ASC, types: [GAME_CHANGE], \
               momentRequestType: VIDEO_CHAPTER_MARKERS) { edges { node { \
               positionMilliseconds \
               details { ... on GameChangeMomentDetails { game { id displayName } } } \
               } } } } }",
};

pub fn variables(id: &VideoId) -> Value {
    json!({ "id": id.as_str() })
}

/// Decodes a response. An unknown VOD (`"video": null`) is `NotFound`; one
/// without moments has no chapters.
pub fn parse(body: &[u8]) -> Result<Chapters, TwitchError> {
    map(super::decode(body, QUERY.operation)?)
}

fn map(data: Data) -> Result<Chapters, TwitchError> {
    let video = data.video.ok_or(TwitchError::NotFound)?;
    let edges = video.moments.and_then(|m| m.edges).unwrap_or_default();
    Ok(Chapters::new(
        edges
            .into_iter()
            .filter_map(|edge| edge?.node?.into_chapter())
            .collect(),
    ))
}

impl Twitch {
    /// The chapters of VOD `id`: the games it went through, from where.
    pub async fn video_chapters(&self, id: &VideoId) -> Result<Chapters, TwitchError> {
        map(self.query(&QUERY, variables(id)).await?)
    }
}

#[derive(Deserialize)]
struct Data {
    video: Option<Video>,
}

#[derive(Deserialize)]
struct Video {
    moments: Option<Moments>,
}

#[derive(Deserialize)]
struct Moments {
    edges: Option<Vec<Option<Edge>>>,
}

#[derive(Deserialize)]
struct Edge {
    node: Option<Moment>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Moment {
    position_milliseconds: Option<u64>,
    details: Option<MomentDetails>,
}

#[derive(Deserialize)]
struct MomentDetails {
    game: Option<GameNode>,
}

impl Moment {
    fn into_chapter(self) -> Option<Chapter> {
        let seconds = u32::try_from(self.position_milliseconds? / 1000).ok()?;
        Some(Chapter {
            start: Seconds::new(seconds),
            game: self.details?.game?.into_game_ref()?,
        })
    }
}
