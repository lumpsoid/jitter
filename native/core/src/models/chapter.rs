use super::{GameRef, Seconds};

/// One stretch of a VOD in one game: Twitch's game-change moment, from
/// `start` until the next chapter's start or the VOD's end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chapter {
    pub start: Seconds,
    pub game: GameRef,
}
