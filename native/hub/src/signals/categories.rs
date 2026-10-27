use rinf::{DartSignal, RustSignal, SignalPiece};
use serde::{Deserialize, Serialize};

use super::StreamCard;

// --- The Categories page ---------------------------------------------------

/// The categories holder is subscribed: answer with a `Snapshot` of the
/// top categories and the `FromYourChannels` strip.
#[derive(Deserialize, DartSignal)]
pub struct CategoriesReady;

/// Start the top categories over from the first page (pull to refresh, or
/// the Retry after a failed first page).
#[derive(Deserialize, DartSignal)]
pub struct RefreshCategories;

/// Load the page after the listed categories (the grid scrolled near its
/// end, or the Retry after a failed next page). Rust ignores it while a
/// page is loading or when there is none, so Dart may send it freely.
#[derive(Deserialize, DartSignal)]
pub struct NextCategoriesPage;

/// The Categories page: the top categories as a snapshot, then diffs, and
/// the strip of categories the live favorites are streaming. One type, so
/// a snapshot and the diffs after it arrive in order.
#[derive(Serialize, RustSignal)]
pub enum CategoriesUpdate {
    /// The whole top categories list. Replaces it.
    Snapshot { state: CategoriesState },
    /// Append `categories` to the loaded list (none when only the footer
    /// changed) and show `more` as its footer.
    Appended {
        categories: Vec<CategoryTile>,
        more: CategoriesMore,
    },
    /// The categories the live favorites are streaming now, in order.
    /// Replaces the strip; empty hides it.
    FromYourChannels {
        categories: Vec<FavoritesCategoryRow>,
    },
}

/// Where the top categories stand.
#[derive(Serialize, SignalPiece)]
pub enum CategoriesState {
    /// Nothing asked for yet.
    Idle,
    /// The first page is loading.
    Loading,
    /// The first page failed; there is nothing to show but `message`.
    Failed { message: String },
    /// The pages so far, in order, and the state of the next one.
    Loaded {
        categories: Vec<CategoryTile>,
        more: CategoriesMore,
    },
}

/// The footer under the loaded categories.
#[derive(Serialize, SignalPiece)]
pub enum CategoriesMore {
    /// Nothing loading: no footer.
    Idle,
    /// The next page is loading: a spinner.
    Loading,
    /// The next page failed: `message` with a Retry that sends
    /// `NextCategoriesPage`. The categories above stay.
    Failed { message: String },
}

/// One top category's tile. `box_art_url` is sized for a tile (3:4),
/// `viewers_label` is formatted (`12.3K viewers`). Tapping it opens the
/// category by `id`.
#[derive(Serialize, SignalPiece)]
pub struct CategoryTile {
    pub id: String,
    pub name: String,
    pub box_art_url: String,
    pub viewers_label: String,
}

/// A category some live favorites are streaming. Tapping it opens the
/// category by `id`.
#[derive(Serialize, SignalPiece)]
pub struct FavoritesCategoryRow {
    pub id: String,
    pub name: String,
    /// The favorites streaming it, most-watched first.
    pub channels: Vec<String>,
}

// --- One category's page ---------------------------------------------------

/// Show category `id`'s live streams, from the first page. Answered with
/// a `CategoryUpdate::Snapshot`: there is nothing to show before a page
/// asks, so this is the category page's ready signal.
#[derive(Deserialize, DartSignal)]
pub struct OpenCategory {
    pub id: String,
}

/// Start the open category over from the first page (pull to refresh, or
/// the Retry after a failed first page).
#[derive(Deserialize, DartSignal)]
pub struct RefreshCategory;

/// Load the open category's next page. Rust ignores it while a page is
/// loading or when there is none, so Dart may send it freely.
#[derive(Deserialize, DartSignal)]
pub struct NextCategoryPage;

/// The open category's streams: a snapshot, then diffs.
#[derive(Serialize, RustSignal)]
pub enum CategoryUpdate {
    /// The whole list. Replaces it.
    Snapshot { state: CategoryState },
    /// Append `streams` after the loaded ones (none when only the footer
    /// changed) and show `more` as its footer. The pinned favorites stay.
    Appended {
        streams: Vec<StreamCard>,
        more: CategoryMore,
    },
}

/// Where the open category's streams stand.
#[derive(Serialize, SignalPiece)]
pub enum CategoryState {
    /// Nothing asked for yet.
    Idle,
    /// The first page is loading.
    Loading,
    /// The first page failed; there is nothing to show but `message`.
    Failed { message: String },
    /// The favorites playing it, then the other streams so far, and the
    /// state of the next page. No channel is in both.
    Loaded {
        favorites: Vec<StreamCard>,
        streams: Vec<StreamCard>,
        more: CategoryMore,
    },
}

/// The footer under the open category's streams.
#[derive(Serialize, SignalPiece)]
pub enum CategoryMore {
    /// Nothing loading: no footer.
    Idle,
    /// The next page is loading: a spinner.
    Loading,
    /// The next page failed: `message` with a Retry that sends
    /// `NextCategoryPage`. The streams above stay.
    Failed { message: String },
}
