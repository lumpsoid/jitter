//! The Dart contract: one file per feature, DTOs shaped for the GUI.
//! Run `rinf gen` after changing anything here.
//!
//! `rinf gen` reads every signal type in the crate into one flat namespace,
//! so names are unique across files and avoid Flutter's own.

mod boot;
// Browse
mod browse;
// Categories
mod categories;
// Chat
mod chat;
// Favorites
mod favorites;
// Playback
mod playback;
// Channel
mod channel;
// Search
mod search;

pub use boot::{Boot, BootFailed, Booted};
// Browse
pub use browse::{
    BrowseMore, BrowseReady, BrowseState, BrowseUpdate, NextBrowsePage, RefreshBrowse, StreamCard,
};
// Categories
pub use categories::{
    CategoriesMore, CategoriesReady, CategoriesState, CategoriesUpdate, CategoryMore,
    CategoryState, CategoryTile, CategoryUpdate, FavoritesCategoryRow, NextCategoriesPage,
    NextCategoryPage, OpenCategory, RefreshCategories, RefreshCategory,
};
// Chat
pub use chat::{ChatFragment, ChatLine, ChatReady, ChatStatus, ChatUpdate, EmoteImage, SetTheme};
// Favorites
pub use favorites::{
    AddFavorite, FavoriteChannel, FavoritesReady, FavoritesUpdate, LiveFavorite, RefreshFavorites,
    RemoveFavorite,
};
// Playback
pub use playback::{
    LeavePlayer, Play, PlaybackTarget, PlaybackUpdate, PlayerCommand, PlayerPosition,
    QualityOption, SelectQuality,
};
// Channel
pub use channel::{
    ChannelLive, ChannelUpdate, OpenChannel, RefreshChannel, SelectVodKind, VodKind, VodTile,
    VodsMore, VodsNextPage, VodsState, VodsUpdate,
};
// Search
pub use search::{
    ChannelResult, NextSearchPage, RefreshSearch, SearchCategoriesState, SearchChannelsState,
    SearchList, SearchMore, SearchTyped, SearchUpdate,
};
