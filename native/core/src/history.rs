//! The watch history: a timeline of visits, kept locally. The player's
//! position reports drive it; `HistoryPolicy` decides what is a visit and
//! when one carries on, and this controller writes through.

use std::collections::HashSet;

use crate::history_policy::HistoryPolicy;
use crate::models::{NewVisit, Timestamp, Visit, VisitDetails, VisitId};
use crate::playback::{PlaybackController, PlaybackState, PlaybackTarget};
use crate::ports::{HistoryStore, StoreError};

/// What a report or a removal did to the history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryChange {
    Unchanged,
    /// Only the visit playing now moved its `last_seen_at`.
    Touched,
    /// The list changed: a visit was added, carried on, or removed.
    Listed,
}

/// What the player is doing for the history.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Watching {
    Nothing,
    /// Playing `target` since `since`, not long enough to count yet.
    Pending {
        target: PlaybackTarget,
        since: Timestamp,
        heard: Timestamp,
    },
    /// Playing `target` as the visit `id`, last reported at `heard` and
    /// last written at `written`.
    Recording {
        target: PlaybackTarget,
        id: VisitId,
        heard: Timestamp,
        written: Timestamp,
    },
}

/// Owner of the history, newest start first.
pub struct HistoryController {
    visits: Vec<Visit>,
    store: Box<dyn HistoryStore>,
    watching: Watching,
}

impl HistoryController {
    /// Starts empty; call [`HistoryController::load`] to read the store.
    pub fn new(store: Box<dyn HistoryStore>) -> Self {
        Self {
            visits: Vec::new(),
            store,
            watching: Watching::Nothing,
        }
    }

    /// Replaces the list with the stored visits, picking up other
    /// processes' writes.
    pub fn load(&mut self) -> Result<(), StoreError> {
        self.visits = self.store.load_visits()?;
        Ok(())
    }

    /// Every visit, newest start first: the timeline.
    pub fn visits(&self) -> &[Visit] {
        &self.visits
    }

    /// The latest visit to each target, newest first: what was watched,
    /// each once.
    pub fn unique(&self) -> Vec<&Visit> {
        let mut seen = HashSet::new();
        self.visits
            .iter()
            .filter(|v| seen.insert(&v.target))
            .collect()
    }

    /// The player showing `target` reported at `now`. Counts only while
    /// `playback` is playing it ([`PlaybackController::playing`]). Starts
    /// timing a target newly playing, stores it as a visit once it counts
    /// (carrying the last visit on when the policy says so), then keeps
    /// its `last_seen_at` up to date. A failed write leaves the controller
    /// as it was, so the next report tries again.
    pub fn heard(
        &mut self,
        playback: &PlaybackController,
        target: &PlaybackTarget,
        now: Timestamp,
    ) -> Result<HistoryChange, StoreError> {
        if !playback.playing(target) {
            return Ok(HistoryChange::Unchanged);
        }
        match self.watching.clone() {
            Watching::Pending {
                target: pending,
                since,
                heard,
            } if pending == *target && !HistoryPolicy::broke_off(heard, now) => {
                if HistoryPolicy::counts(since, now) {
                    return self.commit(playback, target, since, now);
                }
                self.watching = Watching::Pending {
                    target: pending,
                    since,
                    heard: now,
                };
                Ok(HistoryChange::Unchanged)
            }
            Watching::Recording {
                target: recording,
                id,
                heard,
                written,
            } if recording == *target && !HistoryPolicy::broke_off(heard, now) => {
                if !HistoryPolicy::touch_due(written, now) {
                    self.watching = Watching::Recording {
                        target: recording,
                        id,
                        heard: now,
                        written,
                    };
                    return Ok(HistoryChange::Unchanged);
                }
                self.touch(id, target, now)
            }
            _ => {
                let ended = self.end_recording();
                self.watching = Watching::Pending {
                    target: target.clone(),
                    since: now,
                    heard: now,
                };
                ended
            }
        }
    }

    /// The player showing `target` was left at `now`: the visit playing
    /// it is written once more, as of now while it was playing and as of
    /// its last report otherwise, and the history waits for the next
    /// target. A target not counted yet is dropped.
    pub fn leave(
        &mut self,
        playback: &PlaybackController,
        target: &PlaybackTarget,
        now: Timestamp,
    ) -> Result<HistoryChange, StoreError> {
        if self.watching_target() != Some(target) {
            return Ok(HistoryChange::Unchanged);
        }
        if let Watching::Recording { heard, .. } = &mut self.watching
            && playback.playing(target)
            && !HistoryPolicy::broke_off(*heard, now)
        {
            *heard = now;
        }
        self.end_recording()
    }

    /// Removes the visit `id`. A failed write leaves the list unchanged.
    pub fn remove(&mut self, id: VisitId) -> Result<(), StoreError> {
        self.store.remove_visit(id)?;
        self.visits.retain(|v| v.id != id);
        if matches!(self.watching, Watching::Recording { id: recording, .. } if recording == id) {
            self.watching = Watching::Nothing;
        }
        Ok(())
    }

    /// Removes every visit to `target`. A failed write leaves the list
    /// unchanged.
    pub fn remove_target(&mut self, target: &PlaybackTarget) -> Result<(), StoreError> {
        self.store.remove_visits_to(target)?;
        self.visits.retain(|v| v.target != *target);
        if self.watching_target() == Some(target) {
            self.watching = Watching::Nothing;
        }
        Ok(())
    }

    /// Removes every visit. A failed write leaves the list unchanged.
    pub fn clear(&mut self) -> Result<(), StoreError> {
        self.store.clear_visits()?;
        self.visits.clear();
        self.watching = Watching::Nothing;
        Ok(())
    }

    fn watching_target(&self) -> Option<&PlaybackTarget> {
        match &self.watching {
            Watching::Nothing => None,
            Watching::Pending { target, .. } | Watching::Recording { target, .. } => Some(target),
        }
    }

    /// Playing `target` since `since` counts: carries the latest visit
    /// on, or stores a new one with what `playback` shows.
    fn commit(
        &mut self,
        playback: &PlaybackController,
        target: &PlaybackTarget,
        since: Timestamp,
        now: Timestamp,
    ) -> Result<HistoryChange, StoreError> {
        let carried = match self.store.latest_visit()? {
            Some(latest) if HistoryPolicy::carries_on(&latest, target, since) => {
                // Removed meanwhile by another head: stays removed.
                self.store.touch_visit(latest.id, now)?.then_some(latest.id)
            }
            _ => None,
        };
        let id = match carried {
            Some(id) => id,
            None => self.store.add_visit(&NewVisit {
                target: target.clone(),
                started_at: since,
                last_seen_at: now,
                details: details(playback),
            })?,
        };
        self.watching = Watching::Recording {
            target: target.clone(),
            id,
            heard: now,
            written: now,
        };
        self.load()?;
        Ok(HistoryChange::Listed)
    }

    /// Writes the visit `id`, playing `target`, as seen at `now`. One
    /// removed meanwhile, here or by another head, stays removed: the
    /// history reloads and playing on starts the next visit.
    fn touch(
        &mut self,
        id: VisitId,
        target: &PlaybackTarget,
        now: Timestamp,
    ) -> Result<HistoryChange, StoreError> {
        let change = self.write_seen(id, now)?;
        self.watching = match change {
            HistoryChange::Touched => Watching::Recording {
                target: target.clone(),
                id,
                heard: now,
                written: now,
            },
            HistoryChange::Unchanged | HistoryChange::Listed => Watching::Nothing,
        };
        Ok(change)
    }

    /// Stops watching. A visit recording is written as of its last
    /// report first, so the time since its last write is not lost. On a
    /// failed write that time is lost, and the history still moves on.
    fn end_recording(&mut self) -> Result<HistoryChange, StoreError> {
        match std::mem::replace(&mut self.watching, Watching::Nothing) {
            Watching::Recording {
                id, heard, written, ..
            } if heard > written => self.write_seen(id, heard),
            _ => Ok(HistoryChange::Unchanged),
        }
    }

    /// Moves the visit `id`'s `last_seen_at` to `at`, in the store and
    /// here: `Touched`, or `Listed` when it was removed meanwhile and the
    /// history reloaded.
    fn write_seen(&mut self, id: VisitId, at: Timestamp) -> Result<HistoryChange, StoreError> {
        if !self.store.touch_visit(id, at)? {
            self.load()?;
            return Ok(HistoryChange::Listed);
        }
        if let Some(visit) = self.visits.iter_mut().find(|v| v.id == id) {
            visit.last_seen_at = visit.last_seen_at.max(at);
        }
        Ok(HistoryChange::Touched)
    }
}

/// What `playback` shows about the target it has ready; empty when it has
/// none.
fn details(playback: &PlaybackController) -> VisitDetails {
    match playback.state() {
        PlaybackState::Ready { details, .. } => VisitDetails::from(&**details),
        _ => VisitDetails::default(),
    }
}
