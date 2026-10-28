use super::NoRewind;

/// What a rewind result did, for the head to tell the user once
/// ([`crate::playback::PlaybackController::apply_rewind`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RewindOutcome {
    /// The recording VOD plays.
    Rewound,
    /// The rewind was refused; the live stream keeps playing and the
    /// control is off, saying why.
    Refused(NoRewind),
    /// It failed with this reason; the live stream keeps playing and the
    /// rewind can be tried again.
    Failed(String),
}
