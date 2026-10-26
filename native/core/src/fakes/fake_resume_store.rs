use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::models::{ResumePoint, VideoId};
use crate::ports::{ResumeStore, StoreError};

/// In-memory [`ResumeStore`]. Can be told to fail loads or writes.
#[derive(Debug, Clone, Default)]
pub struct FakeResumeStore {
    points: Arc<Mutex<Vec<ResumePoint>>>,
    fail_loads: bool,
    fail_writes: bool,
}

impl FakeResumeStore {
    /// A store already holding `points`.
    pub fn with_points(points: Vec<ResumePoint>) -> Self {
        Self {
            points: Arc::new(Mutex::new(points)),
            ..Self::default()
        }
    }

    /// Every `get` returns `Err`.
    pub fn failing_loads(self) -> Self {
        Self {
            fail_loads: true,
            ..self
        }
    }

    /// Every `put` and `clear` returns `Err` and stores nothing.
    pub fn failing_writes(self) -> Self {
        Self {
            fail_writes: true,
            ..self
        }
    }

    /// What is stored now.
    pub fn points(&self) -> Vec<ResumePoint> {
        self.lock().clone()
    }

    fn lock(&self) -> MutexGuard<'_, Vec<ResumePoint>> {
        self.points.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn check_write(&self) -> Result<(), StoreError> {
        if self.fail_writes {
            return Err(StoreError::new("fake write failure"));
        }
        Ok(())
    }
}

impl ResumeStore for FakeResumeStore {
    fn get(&self, id: &VideoId) -> Result<Option<ResumePoint>, StoreError> {
        if self.fail_loads {
            return Err(StoreError::new("fake load failure"));
        }
        Ok(self.lock().iter().find(|p| p.video_id == *id).cloned())
    }

    fn put(&mut self, point: &ResumePoint) -> Result<(), StoreError> {
        self.check_write()?;
        let mut points = self.lock();
        points.retain(|p| p.video_id != point.video_id);
        points.push(point.clone());
        Ok(())
    }

    fn clear(&mut self, id: &VideoId) -> Result<(), StoreError> {
        self.check_write()?;
        self.lock().retain(|p| p.video_id != *id);
        Ok(())
    }
}
