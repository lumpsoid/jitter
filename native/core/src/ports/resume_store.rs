use crate::models::{ResumePoint, VideoId};
use crate::ports::StoreError;

/// Persistent VOD resume points, one per video. Owned by `ResumeController`.
/// Last writer wins: that is the right answer for "where did I stop".
pub trait ResumeStore: Send {
    fn get(&self, id: &VideoId) -> Result<Option<ResumePoint>, StoreError>;

    /// Stores `point`, replacing any point for the same video.
    fn put(&mut self, point: &ResumePoint) -> Result<(), StoreError>;

    /// Deletes the point for `id`. An unknown id is not an error.
    fn clear(&mut self, id: &VideoId) -> Result<(), StoreError>;
}
