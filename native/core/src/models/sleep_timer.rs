use std::time::{Duration, Instant};

use super::FadeLevel;

/// Over its last `SLEEP_FADE` a sleep timer fades the volume from the
/// user's own down to silence.
pub const SLEEP_FADE: Duration = Duration::from_secs(30);

/// How often the fade steps the volume down: often enough to sound smooth.
pub const SLEEP_FADE_STEP: Duration = Duration::from_millis(250);

/// The time left is shown in whole minutes, rounded up.
const MINUTE: Duration = Duration::from_secs(60);

/// A countdown to stopping playback. Core never reads the clock: heads pass
/// `now` (a monotonic instant) and wake it at [`Self::wake`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SleepTimer {
    deadline: Instant,
    /// When the time left or the fade next changes.
    wake: Instant,
    /// The time left as of the last tick, in whole minutes rounded up.
    minutes_left: u32,
}

/// What a [`SleepTimer::tick`] found.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SleepStep {
    /// Still counting. `relabeled`: [`SleepTimer::minutes_left`] changed.
    /// `fade`: the level to play at now, `None` before the fade starts.
    Counting {
        relabeled: bool,
        fade: Option<FadeLevel>,
    },
    /// The deadline passed.
    Expired,
}

impl SleepTimer {
    /// A timer `duration` from `now`; `None` for a zero duration or one
    /// past what the clock can hold.
    pub fn arm(duration: Duration, now: Instant) -> Option<Self> {
        if duration.is_zero() {
            return None;
        }
        let deadline = now.checked_add(duration)?;
        Some(Self {
            deadline,
            wake: next_wake(deadline, now),
            minutes_left: minutes(duration),
        })
    }

    /// The time left at `now`; zero once expired.
    pub fn remaining(&self, now: Instant) -> Duration {
        self.deadline.saturating_duration_since(now)
    }

    pub fn expired(&self, now: Instant) -> bool {
        self.remaining(now).is_zero()
    }

    /// The level to play at `now`: the user's volume until the last
    /// [`SLEEP_FADE`], then linearly down to silence at the deadline.
    pub fn fade_level(&self, now: Instant) -> FadeLevel {
        let remaining = self.remaining(now);
        if remaining >= SLEEP_FADE {
            return FadeLevel::FULL;
        }
        FadeLevel::new(remaining.as_secs_f64() / SLEEP_FADE.as_secs_f64())
    }

    /// The time left as of the last tick, in whole minutes rounded up: a
    /// timer with 30 seconds left shows 1.
    pub fn minutes_left(&self) -> u32 {
        self.minutes_left
    }

    /// When the head should next call [`Self::tick`]: the next minute the
    /// time left changes, the fade's start, each fade step, the deadline.
    pub fn wake(&self) -> Instant {
        self.wake
    }

    /// Advances to `now`: what changed since the last tick.
    pub fn tick(&mut self, now: Instant) -> SleepStep {
        let remaining = self.remaining(now);
        if remaining.is_zero() {
            return SleepStep::Expired;
        }
        let minutes_left = minutes(remaining);
        let relabeled = minutes_left != self.minutes_left;
        self.minutes_left = minutes_left;
        self.wake = next_wake(self.deadline, now);
        SleepStep::Counting {
            relabeled,
            fade: (remaining <= SLEEP_FADE).then(|| self.fade_level(now)),
        }
    }
}

/// Whole minutes in `duration`, rounded up.
fn minutes(duration: Duration) -> u32 {
    let seconds = duration.as_secs() + u64::from(duration.subsec_nanos() > 0);
    u32::try_from(seconds.div_ceil(60)).unwrap_or(u32::MAX)
}

/// The first instant after `now` at which something shows: a fade step
/// while fading, else the fade's start or the next minute, whichever is
/// sooner; never past `deadline`.
fn next_wake(deadline: Instant, now: Instant) -> Instant {
    let remaining = deadline.saturating_duration_since(now);
    let wait = if remaining <= SLEEP_FADE {
        SLEEP_FADE_STEP
    } else {
        // The label drops a minute once the time left reaches the last
        // whole minute below it.
        let below = MINUTE.saturating_mul(minutes(remaining).saturating_sub(1));
        let to_next_minute = remaining.saturating_sub(below);
        to_next_minute.min(remaining.saturating_sub(SLEEP_FADE))
    };
    now.checked_add(wait)
        .map_or(deadline, |at| at.min(deadline))
}
