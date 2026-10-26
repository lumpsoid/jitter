//! When to resume a VOD and when a watched position is worth saving. Not
//! every stored position is restored: jitter does not resume a VOD barely
//! started, and forgets one that was finished.

use crate::models::Seconds;

/// Below this position a VOD starts from the beginning.
pub const MIN_RESUME: Seconds = Seconds::new(30);

/// A VOD is finished in its last `FINISHED_TAIL` or past
/// `FINISHED_PERCENT` of its duration, whichever is closer to the end.
pub const FINISHED_TAIL: Seconds = Seconds::new(60);
pub const FINISHED_PERCENT: u64 = 95;

/// Saves are throttled: a position that moved less than this, in either
/// direction, since the last save is not saved again.
pub const SAVE_STEP: Seconds = Seconds::new(2);

/// What to do with a reported playback position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResumeDecision {
    Save,
    /// The VOD is finished: forget its point.
    Clear,
    Skip,
}

pub struct ResumePolicy;

impl ResumePolicy {
    /// Where to start a VOD whose stored point is `position` of `duration`.
    pub fn start_position(position: Seconds, duration: Seconds) -> Seconds {
        if position < MIN_RESUME || Self::is_finished(position, duration) {
            Seconds::ZERO
        } else {
            position
        }
    }

    /// Finished once past both the tail threshold and the percentage
    /// threshold, i.e. past whichever is closer to the end. On a VOD shorter
    /// than the tail, the tail threshold is 0 and the percentage decides.
    /// A position past the duration is finished.
    pub fn is_finished(position: Seconds, duration: Seconds) -> bool {
        let past_tail = position >= duration - FINISHED_TAIL;
        let past_percent =
            u64::from(position.get()) * 100 >= u64::from(duration.get()) * FINISHED_PERCENT;
        past_tail && past_percent
    }

    /// `last_saved` is the stored position for this VOD, `None` when
    /// nothing is stored.
    pub fn decide(
        position: Seconds,
        duration: Seconds,
        last_saved: Option<Seconds>,
    ) -> ResumeDecision {
        if Self::is_finished(position, duration) {
            return match last_saved {
                Some(_) => ResumeDecision::Clear,
                None => ResumeDecision::Skip,
            };
        }
        match last_saved {
            Some(last) if moved(last, position) < SAVE_STEP => ResumeDecision::Skip,
            _ => ResumeDecision::Save,
        }
    }
}

fn moved(from: Seconds, to: Seconds) -> Seconds {
    if to > from { to - from } else { from - to }
}
