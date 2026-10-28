use super::{Channel, GameRef, Seconds, Timestamp, VideoId};

/// Broadcast types we list. Twitch's video nodes do not carry one: a video's
/// kind is the type it was requested as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoKind {
    Archive,
    Highlight,
    Upload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Video {
    pub id: VideoId,
    pub channel: Channel,
    pub title: String,
    pub duration: Seconds,
    pub published_at: Timestamp,
    pub thumbnail_url: String,
    pub kind: VideoKind,
    /// The category it was in, when Twitch names one.
    pub game: Option<GameRef>,
}
