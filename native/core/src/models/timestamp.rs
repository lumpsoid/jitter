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
}
