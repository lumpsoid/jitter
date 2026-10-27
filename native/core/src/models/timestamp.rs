use super::Seconds;

/// A point in time as Unix seconds. Core has no time crate; twitch parses
/// Twitch's RFC 3339 strings into this and heads pass `now`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(i64);

impl Timestamp {
    pub fn from_unix(seconds: i64) -> Self {
        Self(seconds)
    }

    pub fn unix(self) -> i64 {
        self.0
    }

    /// The time from `start` to `self`. A `start` after `self` (clock
    /// skew) reads as zero, and a span beyond `u32` seconds saturates.
    pub fn since(self, start: Timestamp) -> Seconds {
        let elapsed = self.0.saturating_sub(start.0).max(0);
        Seconds::new(u32::try_from(elapsed).unwrap_or(u32::MAX))
    }
}
