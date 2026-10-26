use super::{Seconds, Timestamp, VideoId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumePoint {
    pub video_id: VideoId,
    pub position: Seconds,
    pub duration: Seconds,
    pub saved_at: Timestamp,
}
