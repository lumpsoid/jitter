use rinf::{DartSignal, RustSignal, SignalPiece};
use serde::{Deserialize, Serialize};

/// The favorites holder is subscribed: answer with a `Snapshot`, then check
/// which favorites are live.
#[derive(Deserialize, DartSignal)]
pub struct FavoritesReady;

/// Look the channel `login` up and keep it as a favorite (a star toggle).
/// The list arrives as a `Snapshot`, or the failure as `Failed`.
#[derive(Deserialize, DartSignal)]
pub struct AddFavorite {
    pub login: String,
}

/// Stop keeping the channel `channel_id` (a star toggle).
#[derive(Deserialize, DartSignal)]
pub struct RemoveFavorite {
    pub channel_id: String,
}

/// Re-read the stored favorites, picking up another head's changes, then
/// check which are live (pull-to-refresh).
#[derive(Deserialize, DartSignal)]
pub struct RefreshFavorites;

/// The favorites list: a snapshot after every change. One type, so a
/// snapshot and the failures around it arrive in order.
#[derive(Serialize, RustSignal)]
pub enum FavoritesUpdate {
    /// Every favorite, in list order (live by viewers, then the rest by
    /// name). Replaces the list.
    Snapshot { favorites: Vec<FavoriteRow> },
    /// An add, remove, reload or live check failed; the list is as last
    /// sent. A one-shot notice for a snackbar, not list state.
    Failed { message: String },
}

/// One favorite channel as the list shows it.
#[derive(Serialize, SignalPiece)]
pub struct FavoriteRow {
    pub channel_id: String,
    pub login: String,
    pub display_name: String,
    /// Known only while the channel is live: the store keeps no avatar.
    pub avatar_url: Option<String>,
    pub status: FavoriteStatus,
}

/// Whether a favorite is live, as of the last live check.
#[derive(Serialize, SignalPiece)]
pub enum FavoriteStatus {
    /// Not checked since it was loaded or added.
    Unknown,
    Offline,
    Live {
        /// `1234 viewers`.
        viewers_label: String,
        title: String,
    },
}
