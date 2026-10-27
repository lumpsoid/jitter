use super::Open;

/// What the player does as the app goes to the background
/// ([`crate::playback::PlaybackController::background`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackgroundStep {
    /// Carry on with this instead: the audio-only quality.
    Open(Open),
    /// Stop until the app is back in front.
    Stop,
}
