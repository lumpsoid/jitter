use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::models::{NewVisit, Timestamp, Visit, VisitId};
use crate::playback::PlaybackTarget;
use crate::ports::{HistoryStore, StoreError};

#[derive(Debug, Default)]
struct Contents {
    visits: Vec<Visit>,
    /// The last id given out; ids are never reused, as in SQLite's
    /// `AUTOINCREMENT`.
    last_id: i64,
}

/// In-memory [`HistoryStore`]. Can be told to fail loads or writes.
#[derive(Debug, Clone, Default)]
pub struct FakeHistoryStore {
    contents: Arc<Mutex<Contents>>,
    fail_loads: bool,
    fail_writes: bool,
}

impl FakeHistoryStore {
    /// A store already holding `visits`, as another head wrote them.
    pub fn with_visits(visits: Vec<Visit>) -> Self {
        let last_id = visits.iter().map(|v| v.id.get()).max().unwrap_or(0);
        Self {
            contents: Arc::new(Mutex::new(Contents { visits, last_id })),
            ..Self::default()
        }
    }

    /// Every `load_all` and `latest` returns `Err`.
    pub fn failing_loads(self) -> Self {
        Self {
            fail_loads: true,
            ..self
        }
    }

    /// Every write returns `Err` and stores nothing.
    pub fn failing_writes(self) -> Self {
        Self {
            fail_writes: true,
            ..self
        }
    }

    /// What is stored now, newest start first.
    pub fn visits(&self) -> Vec<Visit> {
        let mut visits = self.lock().visits.clone();
        newest_first(&mut visits);
        visits
    }

    fn lock(&self) -> MutexGuard<'_, Contents> {
        self.contents.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn check_load(&self) -> Result<(), StoreError> {
        if self.fail_loads {
            return Err(StoreError::new("fake load failure"));
        }
        Ok(())
    }

    fn check_write(&self) -> Result<(), StoreError> {
        if self.fail_writes {
            return Err(StoreError::new("fake write failure"));
        }
        Ok(())
    }
}

fn newest_first(visits: &mut [Visit]) {
    visits.sort_by_key(|v| std::cmp::Reverse((v.started_at, v.id.get())));
}

impl HistoryStore for FakeHistoryStore {
    fn load_visits(&self) -> Result<Vec<Visit>, StoreError> {
        self.check_load()?;
        Ok(self.visits())
    }

    fn latest_visit(&self) -> Result<Option<Visit>, StoreError> {
        self.check_load()?;
        Ok(self.visits().into_iter().next())
    }

    fn add_visit(&mut self, visit: &NewVisit) -> Result<VisitId, StoreError> {
        self.check_write()?;
        let mut contents = self.lock();
        contents.last_id += 1;
        let id = VisitId::from_raw(contents.last_id);
        contents.visits.push(Visit {
            id,
            target: visit.target.clone(),
            started_at: visit.started_at,
            last_seen_at: visit.last_seen_at,
            details: visit.details.clone(),
        });
        Ok(id)
    }

    fn touch_visit(&mut self, id: VisitId, at: Timestamp) -> Result<bool, StoreError> {
        self.check_write()?;
        let mut contents = self.lock();
        let Some(visit) = contents.visits.iter_mut().find(|v| v.id == id) else {
            return Ok(false);
        };
        visit.last_seen_at = visit.last_seen_at.max(at);
        Ok(true)
    }

    fn remove_visit(&mut self, id: VisitId) -> Result<(), StoreError> {
        self.check_write()?;
        self.lock().visits.retain(|v| v.id != id);
        Ok(())
    }

    fn remove_visits_to(&mut self, target: &PlaybackTarget) -> Result<(), StoreError> {
        self.check_write()?;
        self.lock().visits.retain(|v| v.target != *target);
        Ok(())
    }

    fn clear_visits(&mut self) -> Result<(), StoreError> {
        self.check_write()?;
        self.lock().visits.clear();
        Ok(())
    }
}
