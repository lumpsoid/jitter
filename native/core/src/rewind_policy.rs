//! Whether a live page can rewind into its broadcast's recording VOD, and
//! where in that VOD it starts.

use crate::models::{NoRewind, PlaybackDetails, Rewind, Seconds, VideoId};

/// A rewind starts this far behind the stream's uptime. The recording VOD
/// trails live by about 5-7 s; starting one 10 s segment further back keeps
/// the player off the playlist's growing edge, so it does not stall waiting
/// for a segment not written yet. The user sees the last few seconds again
/// rather than a gap. The VOD also starts about 5 s after the stream's
/// `createdAt`, so the user lands about 25 s behind live, plus the live
/// player's own latency, which is ignored, as is a seek back inside the
/// live buffer: roughly where they were.
pub const REWIND_BEHIND: Seconds = Seconds::new(20);

pub struct RewindPolicy;

impl RewindPolicy {
    /// What a live page offers to rewind into, from the live `details` as
    /// they loaded. `refused` is the VOD Twitch last refused to play: it
    /// stays off.
    pub fn offer(details: &Result<PlaybackDetails, String>, refused: Option<&VideoId>) -> Rewind {
        let Some(live) = details.as_ref().ok().and_then(|d| d.live.as_ref()) else {
            return Rewind::Unavailable(NoRewind::NoDetails);
        };
        let Some(video) = &live.archive else {
            return Rewind::Unavailable(NoRewind::NoRecording);
        };
        if live.started_at.is_none() {
            return Rewind::Unavailable(NoRewind::NoStart);
        }
        if refused == Some(video) {
            return Rewind::Unavailable(NoRewind::Forbidden);
        }
        Rewind::Available {
            video: video.clone(),
        }
    }

    /// Where in the recording VOD to start, `uptime` into the stream:
    /// [`REWIND_BEHIND`] before it, or its start.
    pub fn start(uptime: Seconds) -> Seconds {
        uptime - REWIND_BEHIND
    }
}
