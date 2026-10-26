use super::Emote;

/// A Twitch emote occurrence in a message's text. `start` and `end` are
/// code-point indices, `end` inclusive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmoteSpan {
    pub start: usize,
    pub end: usize,
    pub emote: Emote,
}
