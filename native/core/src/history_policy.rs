//! When playing becomes a history entry, and when it carries on the last
//! one. An entry is a visit, one sitting with one target, not an open: a
//! quality switch, a reconnect, a Retry or a return from the background
//! are the same visit, and so is a return to it after a peek elsewhere too
//! short to count. The history is a timeline, so a visit is never moved:
//! playing an old one again is a new visit today.

use crate::models::{Seconds, Timestamp, Visit};
use crate::playback::PlaybackTarget;

/// Playing shorter than this is a misclick, not a visit.
pub const MIN_WATCH: Seconds = Seconds::new(10);

/// Playing the target of the last visit again within this long of its
/// last report carries that visit on. Within one page, reports this far
/// apart (stopped in the background, slept and resumed) end the visit,
/// and playing on starts the next.
pub const MERGE_GAP: Seconds = Seconds::new(30 * 60);

/// A visit's `last_seen_at` is written at most this often while it plays;
/// leaving the page writes it once more.
pub const TOUCH_STEP: Seconds = Seconds::new(30);

pub struct HistoryPolicy;

impl HistoryPolicy {
    /// Whether playing since `since` has lasted long enough to be a visit.
    pub fn counts(since: Timestamp, now: Timestamp) -> bool {
        now.since(since) >= MIN_WATCH
    }

    /// Whether playing `target` from `since` carries on `latest`, the
    /// visit started last: the same target, last seen no more than
    /// [`MERGE_GAP`] before. Any visit started since would be the latest
    /// instead, so going back after watching something else is a new visit.
    pub fn carries_on(latest: &Visit, target: &PlaybackTarget, since: Timestamp) -> bool {
        latest.target == *target && since.since(latest.last_seen_at) <= MERGE_GAP
    }

    /// Whether reports `heard` and then at `now` are too far apart to be
    /// one sitting.
    pub fn broke_off(heard: Timestamp, now: Timestamp) -> bool {
        now.since(heard) > MERGE_GAP
    }

    /// Whether a visit written at `written` is written again at `now`.
    pub fn touch_due(written: Timestamp, now: Timestamp) -> bool {
        now.since(written) >= TOUCH_STEP
    }
}
