use super::Open;

/// What a stalled stream's reload did, for the head to log
/// ([`crate::playback::PlaybackController::apply_reload`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReloadOutcome {
    /// Resolved afresh: what the player opens.
    Reloaded(Open),
    /// It failed with this reason; the player keeps waiting on what it
    /// had, and the next [`super::STALL_LIMIT`] tries again.
    Failed(String),
}
