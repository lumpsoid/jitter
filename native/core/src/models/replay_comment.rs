use super::{ChatMessage, Seconds};

/// A message from a VOD's chat, and where in the video it was sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayComment {
    /// Seconds into the video.
    pub offset: Seconds,
    pub message: ChatMessage,
}
