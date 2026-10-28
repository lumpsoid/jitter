/// Why resolving a live page's recording VOD failed
/// ([`crate::playback::PlaybackController::apply_rewind`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RewindFailure {
    /// Twitch refused to play it: the control turns off for this VOD.
    Forbidden,
    /// Anything else, timing out included, with the text to show; it can
    /// be tried again.
    Other(String),
}
