/// What an import changed, for telling the user. Settings are not counted:
/// every one the backup holds is taken.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Imported {
    /// Favorites that were not stored.
    pub favorites: usize,
    /// Visits that were not stored.
    pub visits: usize,
    /// Resume points for videos with none stored, or newer than the one
    /// stored.
    pub resume_points: usize,
}
