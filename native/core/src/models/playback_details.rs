use super::{GameRef, Timestamp};

/// What is playing, beside how to play it: what the player page shows
/// about the stream or VOD. Empty when Twitch could not tell.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlaybackDetails {
    /// The category being played: a live stream's current one, a VOD's
    /// recorded one. `None` when none is set or it is not known.
    pub game: Option<GameRef>,
    /// When a VOD was streamed, i.e. its recording began. `None` for a
    /// live stream, or when it is not known.
    pub streamed_at: Option<Timestamp>,
}
