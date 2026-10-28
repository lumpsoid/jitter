use super::{NoRewind, Seconds, VideoId};

/// A live page's rewind: playing the broadcast's recording VOD in place of
/// the live stream, and back. The page stays on its live target throughout;
/// this says which stream it plays and what the control offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rewind {
    /// The control is enabled: this broadcast's recording VOD.
    Available { video: VideoId },
    /// The control is disabled, saying why.
    Unavailable(NoRewind),
    /// The VOD is resolving; the live stream keeps playing.
    Loading { video: VideoId, start: Seconds },
    /// The last try failed (not forbidden); enabled again, with the reason.
    Failed { video: VideoId, message: String },
    /// Playing `video` from `start`: `playback`/`details`/`selected` of
    /// Ready are the VOD's.
    Rewound { video: VideoId, start: Seconds },
    /// Still playing `video` as in `Rewound` while the live stream
    /// resolves; a failure goes back to `Rewound`.
    Returning { video: VideoId, start: Seconds },
}

impl Rewind {
    /// The VOD a rewind resolves from here: `None` unless the control is
    /// enabled, i.e. offered or failed.
    pub fn offered(&self) -> Option<&VideoId> {
        match self {
            Self::Available { video } | Self::Failed { video, .. } => Some(video),
            _ => None,
        }
    }

    /// The VOD the player shows and where it started it, while it does:
    /// rewound, or returning to live.
    pub fn playing_vod(&self) -> Option<(&VideoId, Seconds)> {
        match self {
            Self::Rewound { video, start } | Self::Returning { video, start } => {
                Some((video, *start))
            }
            _ => None,
        }
    }
}
