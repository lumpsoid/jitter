use std::time::{Duration, Instant};

/// A stream the player has waited on this long is reloaded: resolved
/// afresh, which can land on another CDN server, and opened again where it
/// was.
pub const STALL_LIMIT: Duration = Duration::from_secs(15);

/// Whether the player waits for data, as it last reported, and whether a
/// reload is out for it. Core never reads the clock: heads pass `now` (a
/// monotonic instant) and wake it at [`Self::wake`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Stall {
    /// Playing, or not reported waiting.
    #[default]
    Flowing,
    /// Waiting since `since`.
    Waiting { since: Instant },
    /// Waited [`STALL_LIMIT`]; a reload is out.
    Reloading,
}

impl Stall {
    /// The player reported whether it is `buffering` at `now`. A wait
    /// that ends while a reload is out drops the reload: the stream
    /// recovered on its own.
    pub fn report(&mut self, buffering: bool, now: Instant) {
        *self = match (*self, buffering) {
            (Self::Flowing, true) => Self::Waiting { since: now },
            (_, false) => Self::Flowing,
            (waiting, true) => waiting,
        };
    }

    /// When a wait reaches [`STALL_LIMIT`]; `None` unless waiting.
    pub fn wake(&self) -> Option<Instant> {
        match self {
            Self::Waiting { since } => since.checked_add(STALL_LIMIT),
            _ => None,
        }
    }

    /// Starts a reload if the wait has reached [`STALL_LIMIT`] at `now`.
    /// Returns how long it waited when it did.
    pub fn reload(&mut self, now: Instant) -> Option<Duration> {
        let Self::Waiting { since } = *self else {
            return None;
        };
        let waited = now.saturating_duration_since(since);
        if waited < STALL_LIMIT {
            return None;
        }
        *self = Self::Reloading;
        Some(waited)
    }

    /// Whether a reload is out.
    pub fn reloading(&self) -> bool {
        *self == Self::Reloading
    }

    /// The reload came back at `now`: the player still waits, on the
    /// stream it reopens or on the one it had, so the count starts over.
    pub fn reloaded(&mut self, now: Instant) {
        *self = Self::Waiting { since: now };
    }
}
