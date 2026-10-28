/// What a back-to-live result did, for the head to tell the user once
/// ([`crate::playback::PlaybackController::apply_return`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReturnOutcome {
    /// The live stream plays again.
    Live,
    /// It failed with this reason; the recording VOD keeps playing and
    /// going back can be tried again.
    Failed(String),
}
