use super::{Channel, Seconds, Timestamp, VideoId};

/// Broadcast types we show. Other types are dropped at the twitch mapping
/// boundary.
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
}
