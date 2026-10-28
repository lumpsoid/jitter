use super::{Seconds, Timestamp, VisitDetails, VisitId};
use crate::playback::PlaybackTarget;

/// One sitting with one live channel or VOD: the history's entry. Opens of
/// the same target with nothing else watched between, close together, are
/// one visit ([`crate::history_policy`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Visit {
    pub id: VisitId,
    pub target: PlaybackTarget,
    pub started_at: Timestamp,
    /// When the player last reported it playing.
    pub last_seen_at: Timestamp,
    pub details: VisitDetails,
}

impl Visit {
    /// How long it was watched, as far as the player reported.
    pub fn watched(&self) -> Seconds {
        self.last_seen_at.since(self.started_at)
    }
}

/// A visit to store; the store gives it its id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewVisit {
    pub target: PlaybackTarget,
    pub started_at: Timestamp,
    pub last_seen_at: Timestamp,
    pub details: VisitDetails,
}
