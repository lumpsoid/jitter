//! A VOD's chat: the web client's persisted `VideoCommentsByOffsetOrCursor`,
//! one page at a time, either from a position in the video or after an
//! earlier page's last comment.

use jitter_core::models::{
    Badge, ChannelId, ChatMessage, Chatter, Cursor, EmoteSpan, Page, ReplayComment, ReplayFrom,
    Rgb, Seconds, VideoId,
};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{PersistedQuery, parse_timestamp};
use crate::client::Twitch;
use crate::emotes::twitch_emote;
use crate::error::TwitchError;

pub const QUERY: PersistedQuery = PersistedQuery {
    operation: "VideoCommentsByOffsetOrCursor",
    hash: "b70a3591ff0f4e0313d126c6a1502d79a1c02baebb288227c582044aa76adf6a",
    needs_device_id: false,
};

/// `contentOffsetSeconds` or `cursor`, never both; Twitch picks the page
/// size.
pub fn variables(video: &VideoId, from: &ReplayFrom) -> Value {
    match from {
        ReplayFrom::Offset(offset) => json!({
            "videoID": video.as_str(),
            "contentOffsetSeconds": offset.get(),
        }),
        ReplayFrom::After(cursor) => json!({
            "videoID": video.as_str(),
            "cursor": cursor.as_str(),
        }),
    }
}

/// Decodes a response body. An unknown VOD (`"video": null`) is
/// `NotFound`; comments missing what a message needs (an id, a commenter
/// core accepts, an offset, a send time) are dropped.
pub fn parse(body: &[u8]) -> Result<Page<ReplayComment>, TwitchError> {
    map(super::decode(body, QUERY.operation)?)
}

fn map(data: Data) -> Result<Page<ReplayComment>, TwitchError> {
    let comments = data
        .video
        .ok_or(TwitchError::NotFound)?
        .comments
        .ok_or_else(|| TwitchError::Decode(format!("{}: comments is null", QUERY.operation)))?;
    let next = next_cursor(&comments);
    let items = comments
        .edges
        .into_iter()
        .filter_map(|edge| edge.node?.into_comment())
        .collect();
    Ok(Page { items, next })
}

impl Twitch {
    /// One page of a VOD's chat, oldest first.
    pub async fn video_comments(
        &self,
        video: &VideoId,
        from: &ReplayFrom,
    ) -> Result<Page<ReplayComment>, TwitchError> {
        map(self.persisted(&QUERY, variables(video, from)).await?)
    }
}

/// The last edge's cursor when Twitch says there is another page, taken
/// before any edge is dropped.
fn next_cursor(comments: &Comments) -> Option<Cursor> {
    if comments.page_info.as_ref()?.has_next_page != Some(true) {
        return None;
    }
    comments.edges.last()?.cursor.clone().map(Cursor::new)
}

#[derive(Deserialize)]
struct Data {
    video: Option<Video>,
}

#[derive(Deserialize)]
struct Video {
    comments: Option<Comments>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Comments {
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
    commenter: Option<Commenter>,
    content_offset_seconds: Option<u32>,
    created_at: Option<String>,
    message: Option<Message>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Commenter {
    id: Option<String>,
    login: Option<String>,
    display_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Message {
    #[serde(default)]
    fragments: Vec<Fragment>,
    #[serde(default)]
    user_badges: Vec<BadgeNode>,
    user_color: Option<String>,
}

#[derive(Deserialize)]
struct Fragment {
    text: Option<String>,
    emote: Option<EmoteNode>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EmoteNode {
    #[serde(rename = "emoteID")]
    emote_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BadgeNode {
    #[serde(rename = "setID")]
    set_id: Option<String>,
    version: Option<String>,
}

impl Node {
    fn into_comment(self) -> Option<ReplayComment> {
        let commenter = self.commenter?;
        let message = self.message?;
        let display_name = commenter
            .display_name
            .filter(|n| !n.is_empty())
            .or(commenter.login)?;
        let (text, emotes) = text_and_emotes(message.fragments);
        Some(ReplayComment {
            offset: Seconds::new(self.content_offset_seconds?),
            message: ChatMessage {
                id: self.id.filter(|id| !id.is_empty())?,
                sender: Chatter {
                    user_id: ChannelId::parse(&commenter.id?).ok()?,
                    display_name,
                    color: message.user_color.as_deref().and_then(Rgb::parse_hex),
                    badges: message
                        .user_badges
                        .into_iter()
                        .filter_map(BadgeNode::into_badge)
                        .collect(),
                },
                text,
                // A replayed `/me` arrives as plain text: Twitch sends no
                // flag for it.
                action: false,
                emotes,
                sent_at: parse_timestamp(&self.created_at?)?,
                deleted: false,
            },
        })
    }
}

impl BadgeNode {
    fn into_badge(self) -> Option<Badge> {
        Some(Badge {
            set_id: self.set_id.filter(|s| !s.is_empty())?,
            version: self.version?,
        })
    }
}

/// The fragments' texts joined, and a span over each emote fragment's text
/// (code points, end inclusive, as live chat's `emotes` tag gives them).
fn text_and_emotes(fragments: Vec<Fragment>) -> (String, Vec<EmoteSpan>) {
    let mut text = String::new();
    let mut spans = Vec::new();
    let mut start = 0;
    for fragment in fragments {
        let Some(piece) = fragment.text else {
            continue;
        };
        let len = piece.chars().count();
        if let Some(id) = fragment.emote.and_then(|e| e.emote_id)
            && len > 0
        {
            spans.push(EmoteSpan {
                start,
                end: start + len - 1,
                emote: twitch_emote(id, piece.clone()),
            });
        }
        text.push_str(&piece);
        start += len;
    }
    (text, spans)
}
