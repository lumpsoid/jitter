use crate::models::{ChannelId, ChannelLogin, Favorite};
use crate::ports::StoreError;

/// Persistent favorite channels, keyed by channel id. Owned by
/// `FavoritesController`, which writes through it before changing its state.
///
/// The database is shared by every head on the machine, so a write must
/// never bring back a row another process removed. That is why there is no
/// upsert: `add` only inserts, `rename` only updates, `remove` only deletes.
pub trait FavoritesStore: Send {
    fn load_all(&self) -> Result<Vec<Favorite>, StoreError>;

    /// Inserts `favorite` if its channel id is not stored. A stored id is
    /// left exactly as it is (its names and `added_at` are not overwritten).
    fn add(&mut self, favorite: &Favorite) -> Result<(), StoreError>;

    /// Updates the cached login and display name of the stored channel `id`.
    /// Never inserts: an id that is not stored (for instance one another
    /// process just removed) stays absent, and that is not an error.
    fn rename(
        &mut self,
        id: &ChannelId,
        login: &ChannelLogin,
        display_name: &str,
    ) -> Result<(), StoreError>;

    /// Deletes the channel `id`. An unknown id is not an error.
    fn remove(&mut self, id: &ChannelId) -> Result<(), StoreError>;
}
