use rinf::{DartSignal, RustSignal, SignalPiece};
use serde::{Deserialize, Serialize};

use super::CategoryTile;

/// The search field's text, sent on every change. Rust waits for typing
/// to pause before it searches, so Dart sends each keystroke as is. Text
/// with nothing but whitespace clears the results.
#[derive(Deserialize, DartSignal)]
pub struct SearchTyped {
    pub text: String,
}

/// Start list `list` over from its first page (pull to refresh, or the
/// Retry after a failed first page). Rust ignores it while nothing is
/// searched.
#[derive(Deserialize, DartSignal)]
pub struct RefreshSearch {
    pub list: SearchList,
}

/// Load the page after list `list`'s results (scrolled near its end, or
/// the Retry after a failed next page). Rust ignores it while a page is
/// loading or when there is none, so Dart may send it freely.
#[derive(Deserialize, DartSignal)]
pub struct NextSearchPage {
    pub list: SearchList,
}

/// The two result lists, in the order the page shows them.
#[derive(Deserialize, SignalPiece)]
pub enum SearchList {
    Channels,
    Categories,
}

/// The search results: each list as a snapshot, then diffs. One type, so
/// a snapshot and the diffs after it arrive in order.
#[derive(Serialize, RustSignal)]
pub enum SearchUpdate {
    /// The whole channel list. Replaces it.
    Channels { state: SearchChannelsState },
    /// Append `channels` to the loaded channels (none when only the footer
    /// changed) and show `more` as its footer.
    ChannelsAppended {
        channels: Vec<ChannelResult>,
        more: SearchMore,
    },
    /// The whole category list. Replaces it.
    Categories { state: SearchCategoriesState },
    /// Append `categories` to the loaded categories (none when only the
    /// footer changed) and show `more` as its footer.
    CategoriesAppended {
        categories: Vec<CategoryTile>,
        more: SearchMore,
    },
}

/// Where the channels matching the search stand.
#[derive(Serialize, SignalPiece)]
pub enum SearchChannelsState {
    /// Nothing searched: the field is empty.
    Idle,
    /// The first page is loading.
    Loading,
    /// The first page failed; there is nothing to show but `message`.
    Failed { message: String },
    /// The pages so far, in Twitch's order (empty: no channel matches),
    /// and the state of the next one.
    Loaded {
        channels: Vec<ChannelResult>,
        more: SearchMore,
    },
}

/// Where the categories matching the search stand.
#[derive(Serialize, SignalPiece)]
pub enum SearchCategoriesState {
    /// Nothing searched: the field is empty.
    Idle,
    /// The first page is loading.
    Loading,
    /// The first page failed; there is nothing to show but `message`.
    Failed { message: String },
    /// The pages so far, in Twitch's order (empty: no category matches),
    /// and the state of the next one.
    Loaded {
        categories: Vec<CategoryTile>,
        more: SearchMore,
    },
}

/// The footer under a loaded result list.
#[derive(Serialize, SignalPiece)]
pub enum SearchMore {
    /// Nothing loading: no footer.
    Idle,
    /// The next page is loading: a spinner.
    Loading,
    /// The next page failed: `message` with a Retry that sends
    /// `NextSearchPage`. The results above stay.
    Failed { message: String },
}

/// One channel a search found. Tapping it opens the channel by `login`.
#[derive(Serialize, SignalPiece)]
pub struct ChannelResult {
    pub login: String,
    pub display_name: String,
    /// `None`: no avatar known, show a placeholder.
    pub avatar_url: Option<String>,
    /// Formatted (`2.9M followers`); `None`: Twitch sent no count.
    pub followers_label: Option<String>,
    /// Formatted (`513 viewers`) while the channel is live; `None`: offline.
    pub live_label: Option<String>,
}
