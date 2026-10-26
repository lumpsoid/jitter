use super::Seconds;

/// The chat room's modes. Starts with everything off.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RoomState {
    pub emote_only: bool,
    pub subs_only: bool,
    pub r9k: bool,
    /// Slow-mode delay; zero is off.
    pub slow: Seconds,
}
