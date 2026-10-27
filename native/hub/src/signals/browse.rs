use rinf::{DartSignal, RustSignal, SignalPiece};
use serde::{Deserialize, Serialize};

/// The top streams holder is subscribed: answer with a `Snapshot`.
#[derive(Deserialize, DartSignal)]
pub struct BrowseReady;

/// Start the top streams over from the first page (pull to refresh, or the
/// Retry after a failed first page).
#[derive(Deserialize, DartSignal)]
pub struct RefreshBrowse;

/// Load the page after the listed ones (the grid scrolled near its end, or
/// the Retry after a failed next page). Rust ignores it while a page is
/// loading or when there is none, so Dart may send it freely.
#[derive(Deserialize, DartSignal)]
pub struct NextBrowsePage;

/// The top streams: a snapshot, then diffs. One type, so the snapshot and
/// the diffs after it arrive in order.
#[derive(Serialize, RustSignal)]
pub enum BrowseUpdate {
    /// The whole list. Replaces it.
    Snapshot { state: BrowseState },
    /// Append `streams` to the loaded list (none when only the footer
    /// changed) and show `more` as its footer.
    Appended {
        streams: Vec<StreamCard>,
        more: BrowseMore,
    },
}

/// Where the top streams stand.
#[derive(Serialize, SignalPiece)]
pub enum BrowseState {
    /// Nothing asked for yet.
    Idle,
    /// The first page is loading.
    Loading,
    /// The first page failed; there is nothing to show but `message`.
    Failed { message: String },
    /// The pages so far, in order, and the state of the next one.
    Loaded {
        streams: Vec<StreamCard>,
        more: BrowseMore,
    },
}

/// The footer under the loaded streams.
#[derive(Serialize, SignalPiece)]
pub enum BrowseMore {
    /// Nothing loading: no footer.
    Idle,
    /// The next page is loading: a spinner.
    Loading,
    /// The next page failed: `message` with a Retry that sends
    /// `NextBrowsePage`. The streams above stay.
    Failed { message: String },
}

/// One live stream's card. Every value is ready to show: `thumbnail_url`
/// is sized for a card, `viewers_label` is formatted (`12.3K viewers`).
/// Tapping it hands `login` on; its `FavoriteStar` reads `channel_id` and
/// `login`.
#[derive(Serialize, SignalPiece)]
pub struct StreamCard {
    pub id: String,
    pub channel_id: String,
    pub login: String,
    pub display_name: String,
    pub title: String,
    pub game_name: Option<String>,
    pub viewers_label: String,
    pub thumbnail_url: String,
}
