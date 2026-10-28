/// Why a live page cannot rewind ([`super::Rewind::Unavailable`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoRewind {
    /// Twitch shows no recording of this broadcast. The channel may still
    /// save its broadcasts: Twitch hides the recording of many of them from
    /// a viewer not signed in.
    NoRecording,
    /// Twitch refused to play the recording without signing in. It does
    /// not say why: subscriber-only, region-blocked, or another refusal.
    Forbidden,
    /// Twitch did not say when the stream started, so there is no position
    /// to rewind to.
    NoStart,
    /// The stream's details did not load.
    NoDetails,
}
