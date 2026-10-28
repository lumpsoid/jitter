use crate::models::{NewVisit, Timestamp, Visit, VisitId};
use crate::playback::PlaybackTarget;
use crate::ports::StoreError;

/// The persistent watch history, append-only: a visit is added and then
/// only extended or removed, never moved. Owned by `HistoryController`,
/// which writes through it before changing its state.
///
/// The database is shared by every head on the machine, so no write may
/// bring back a visit another process removed: `touch_visit` only updates.
pub trait HistoryStore: Send {
    /// Every visit, newest start first.
    fn load_visits(&self) -> Result<Vec<Visit>, StoreError>;

    /// The visit started last, `None` when there is none.
    fn latest_visit(&self) -> Result<Option<Visit>, StoreError>;

    fn add_visit(&mut self, visit: &NewVisit) -> Result<VisitId, StoreError>;

    /// Moves the visit `id`'s `last_seen_at` forward to `at` (never back).
    /// Returns whether the visit is stored: `false` for one removed, which
    /// stays removed.
    fn touch_visit(&mut self, id: VisitId, at: Timestamp) -> Result<bool, StoreError>;

    /// Deletes the visit `id`. An unknown id is not an error.
    fn remove_visit(&mut self, id: VisitId) -> Result<(), StoreError>;

    /// Deletes every visit to `target`.
    fn remove_visits_to(&mut self, target: &PlaybackTarget) -> Result<(), StoreError>;

    /// Deletes every visit.
    fn clear_visits(&mut self) -> Result<(), StoreError>;
}
