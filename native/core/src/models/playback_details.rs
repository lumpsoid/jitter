use super::GameRef;

/// What is playing, beside how to play it: what the player page shows
/// about the stream or VOD. Empty when Twitch could not tell.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlaybackDetails {
    /// The category being played: a live stream's current one, a VOD's
    /// recorded one. `None` when none is set or it is not known.
    pub game: Option<GameRef>,
}
