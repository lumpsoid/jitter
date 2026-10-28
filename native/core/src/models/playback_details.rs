use super::{Channel, Chapters, GameRef, LiveDetails, Seconds, Timestamp};

/// What is playing, beside how to play it: what the player page shows
/// about the stream or VOD. Empty when Twitch could not tell.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlaybackDetails {
    /// The channel being watched: a live stream's broadcaster, a VOD's
    /// owner. `None` when it is not known.
    pub channel: Option<Channel>,
    /// The title its broadcaster gave it: a live stream's current one, a
    /// VOD's own. `None` when none is set or it is not known.
    pub title: Option<String>,
    /// The category being played: a live stream's current one, a VOD's
    /// recorded one. `None` when none is set or it is not known.
    pub game: Option<GameRef>,
    /// A VOD's chapters: the games it went through. Empty for a live
    /// stream, a VOD in one game, or when they are not known.
    pub chapters: Chapters,
    /// When a VOD was streamed, i.e. its recording began. `None` for a
    /// live stream, or when it is not known.
    pub streamed_at: Option<Timestamp>,
    /// A live stream's viewers and start. `None` for a VOD, or when it is
    /// not known.
    pub live: Option<LiveDetails>,
}

impl PlaybackDetails {
    /// The game to show with the player `at` that position (`None` while
    /// it is not known): the chapter's, for a VOD with chapters; `game`
    /// otherwise. A VOD with chapters shows none until its position is
    /// known, rather than one it may not be at.
    pub fn game_at(&self, at: Option<Seconds>) -> Option<&GameRef> {
        if self.chapters.is_empty() {
            return self.game.as_ref();
        }
        self.chapters.game_at(at?)
    }
}
