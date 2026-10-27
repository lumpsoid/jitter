//! The Dart contract: one file per feature, DTOs shaped for the GUI.
//! Run `rinf gen` after changing anything here.
//!
//! `rinf gen` reads every signal type in the crate into one flat namespace,
//! so names are unique across files and avoid Flutter's own.

mod boot;
// Browse
mod browse;
// Chat
mod chat;
// Favorites
mod favorites;
// Playback
mod playback;

pub use boot::{Boot, BootFailed, Booted};
// Browse
pub use browse::{
    BrowseMore, BrowseReady, BrowseState, BrowseUpdate, NextBrowsePage, RefreshBrowse, StreamCard,
};
// Chat
pub use chat::{ChatFragment, ChatLine, ChatReady, ChatStatus, ChatUpdate, EmoteImage, SetTheme};
// Favorites
pub use favorites::{
    AddFavorite, FavoriteRow, FavoriteStatus, FavoritesReady, FavoritesUpdate, RefreshFavorites,
    RemoveFavorite,
};
// Playback
pub use playback::{
    LeavePlayer, Play, PlaybackTarget, PlaybackUpdate, PlayerCommand, PlayerPosition,
    QualityOption, SelectQuality,
};
