//! VOD resume points: where to start a VOD, and recording positions as it
//! plays. `ResumePolicy` decides; this controller loads and writes through.

use std::collections::HashMap;

use crate::models::{ResumePoint, Seconds, Timestamp, VideoId};
use crate::ports::{ResumeStore, StoreError};
use crate::resume_policy::{ResumeDecision, ResumePolicy};

pub struct ResumeController {
    store: Box<dyn ResumeStore>,
    /// The stored position per video as of this controller's last read or
    /// write; `None` when nothing is stored. Seeded from the store on a
    /// video's first `record`.
    last_saved: HashMap<VideoId, Option<Seconds>>,
}

impl ResumeController {
    pub fn new(store: Box<dyn ResumeStore>) -> Self {
        Self {
            store,
            last_saved: HashMap::new(),
        }
    }

    /// Where to start playing `id`, judged against the duration stored with
    /// its point. No point gives 0.
    pub fn start_position(&self, id: &VideoId) -> Result<Seconds, StoreError> {
        Ok(self.store.get(id)?.map_or(Seconds::ZERO, |p| {
            ResumePolicy::start_position(p.position, p.duration)
        }))
    }

    /// Reports the playback position. Heads may call this on every player
    /// tick; the policy throttles. A failed write leaves the controller's
    /// view of the store unchanged and returns `Err`.
    pub fn record(
        &mut self,
        id: VideoId,
        position: Seconds,
        duration: Seconds,
        now: Timestamp,
    ) -> Result<ResumeDecision, StoreError> {
        let last_saved = match self.last_saved.get(&id) {
            Some(last) => *last,
            None => {
                let stored = self.store.get(&id)?.map(|p| p.position);
                self.last_saved.insert(id.clone(), stored);
                stored
            }
        };
        let decision = ResumePolicy::decide(position, duration, last_saved);
        match decision {
            ResumeDecision::Save => {
                self.store.put(&ResumePoint {
                    video_id: id.clone(),
                    position,
                    duration,
                    saved_at: now,
                })?;
                self.last_saved.insert(id, Some(position));
            }
            ResumeDecision::Clear => {
                self.store.clear(&id)?;
                self.last_saved.insert(id, None);
            }
            ResumeDecision::Skip => {}
        }
        Ok(decision)
    }
}
