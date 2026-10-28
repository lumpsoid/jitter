use super::{Seconds, Timestamp, VideoId};

/// What a live stream shows beside its video: how many watch it, and since
/// when it is live, and the VOD recording it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveDetails {
    pub viewers: u32,
    /// When the stream went live; `None` when Twitch did not say.
    pub started_at: Option<Timestamp>,
    /// This broadcast's VOD while it records; `None` when the channel does
    /// not save its broadcasts, or Twitch does not show the recording.
    pub archive: Option<VideoId>,
}

impl LiveDetails {
    /// How long the stream has been live at `now`; `None` without a start
    /// time. Reads like [`super::Stream::uptime`].
    pub fn uptime(&self, now: Timestamp) -> Option<Seconds> {
        Some(now.since(self.started_at?))
    }
}
