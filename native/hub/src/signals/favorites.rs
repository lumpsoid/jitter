use rinf::{DartSignal, RustSignal, SignalPiece};
use serde::{Deserialize, Serialize};

use super::StreamCard;

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
    /// Every favorite, split by the last live check; each favorite is in
    /// exactly one list. Replaces the list.
    Snapshot {
        /// Live, by viewers descending.
        live: Vec<LiveFavorite>,
        /// Checked and offline, by name.
        offline: Vec<FavoriteChannel>,
        /// Not checked since loaded or added, by name: neither live nor
        /// known to be offline.
        unchecked: Vec<FavoriteChannel>,
    },
    /// An add, remove, reload or live check failed; the list is as last
    /// sent. A one-shot notice for a snackbar, not list state.
    Failed { message: String },
}

/// A live favorite as its card shows it.
#[derive(Serialize, SignalPiece)]
pub struct LiveFavorite {
    pub stream: StreamCard,
    pub avatar_url: String,
    /// `3h 12m`, as of the snapshot; `None` when Twitch sent no start time.
    pub uptime_label: Option<String>,
}

/// A favorite that is not live: what the store keeps, with no avatar.
#[derive(Serialize, SignalPiece)]
pub struct FavoriteChannel {
    pub channel_id: String,
    pub login: String,
    pub display_name: String,
}
