use super::Seconds;

/// What a player opens: one quality's playlist, where to start in it, and
/// whether it is a live stream (no seeking, no resume recording).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Open {
    pub url: String,
    pub start: Seconds,
    pub live: bool,
}
