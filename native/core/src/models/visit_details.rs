use super::{Channel, GameRef, PlaybackDetails, Timestamp};

/// What a visit was watching, as it was when the visit started: kept, so
/// last week's visit shows last week's stream and not what the channel
/// shows now. Empty when Twitch could not tell.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VisitDetails {
    pub channel: Option<Channel>,
    pub title: Option<String>,
    pub game: Option<GameRef>,
    /// When a VOD was streamed; `None` for a live stream.
    pub streamed_at: Option<Timestamp>,
}

impl From<&PlaybackDetails> for VisitDetails {
    fn from(details: &PlaybackDetails) -> Self {
        Self {
            channel: details.channel.clone(),
            title: details.title.clone(),
            game: details.game.clone(),
            streamed_at: details.streamed_at,
        }
    }
}
