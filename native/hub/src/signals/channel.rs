use rinf::{DartSignal, RustSignal, SignalPiece};
use serde::{Deserialize, Serialize};

/// A channel page opened for `login`: look the channel up and list its
/// archive. Answered with a `ChannelUpdate` and a `VodsUpdate` snapshot,
/// so it is also the page's ready signal.
#[derive(Deserialize, DartSignal)]
pub struct OpenChannel {
    pub login: String,
}

/// Look the open channel up again and start its VODs over from the first
/// page (pull to refresh, or a Retry).
#[derive(Deserialize, DartSignal)]
pub struct RefreshChannel;

/// List the open channel's VODs of `kind` instead (a tab). Rust ignores
/// the kind already listed, so Dart may send it on every VOD tab it shows.
#[derive(Deserialize, DartSignal)]
pub struct SelectVodKind {
    pub kind: VodKind,
}

/// Load the page after the listed VODs (the list scrolled near its end, or
/// the Retry after a failed next page). Rust ignores it while a page is
/// loading or when there is none, so Dart may send it freely.
#[derive(Deserialize, DartSignal)]
pub struct VodsNextPage;

/// The VOD tabs, in the order the page shows them.
#[derive(Deserialize, SignalPiece)]
pub enum VodKind {
    Archive,
    Highlight,
    Upload,
}

/// The open channel's header, whole, after every change.
#[derive(Serialize, RustSignal)]
pub enum ChannelUpdate {
    /// No channel opened yet.
    Idle,
    /// Looking `login` up.
    Loading { login: String },
    /// No channel has this login.
    NotFound { login: String },
    /// The lookup failed: `message`, with a Retry that sends
    /// `RefreshChannel`.
    Failed { login: String, message: String },
    /// The channel. A `FavoriteStar` for it reads `channel_id` and `login`;
    /// its About tab shows `followers_label` and `description`.
    Found {
        channel_id: String,
        login: String,
        display_name: String,
        avatar_url: String,
        /// The channel's "about" text; `None`: it has none.
        description: Option<String>,
        /// `2.9M followers`; `None`: Twitch sent no count, show nothing.
        followers_label: Option<String>,
        live: ChannelLive,
    },
}

/// Whether the open channel is live.
#[derive(Serialize, SignalPiece)]
pub enum ChannelLive {
    Checking,
    Offline,
    /// Live now; tapping it opens the stream by the channel's login.
    Live {
        title: String,
        game_name: Option<String>,
        /// `12.3K viewers`.
        viewers_label: String,
    },
    Failed {
        message: String,
    },
}

/// The open channel's VODs of the selected kind: a snapshot, then diffs.
/// One type, so the snapshot and the diffs after it arrive in order.
#[derive(Serialize, RustSignal)]
pub enum VodsUpdate {
    /// The whole list. Replaces it.
    Snapshot { state: VodsState },
    /// Append `vods` to the loaded list (none when only the footer
    /// changed) and show `more` as its footer.
    Appended { vods: Vec<VodTile>, more: VodsMore },
}

/// Where the VOD list stands.
#[derive(Serialize, SignalPiece)]
pub enum VodsState {
    /// No channel opened yet.
    Idle,
    /// The first page is loading.
    Loading,
    /// The first page failed; there is nothing to show but `message`.
    Failed { message: String },
    /// The pages so far, newest first, and the state of the next one.
    Loaded { vods: Vec<VodTile>, more: VodsMore },
}

/// The footer under the loaded VODs.
#[derive(Serialize, SignalPiece)]
pub enum VodsMore {
    /// Nothing loading: no footer.
    Idle,
    /// The next page is loading: a spinner.
    Loading,
    /// The next page failed: `message` with a Retry that sends
    /// `VodsNextPage`. The VODs above stay.
    Failed { message: String },
}

/// One VOD's tile. Every value is ready to show. Tapping it hands `id` on.
#[derive(Serialize, SignalPiece)]
pub struct VodTile {
    pub id: String,
    pub title: String,
    /// `1:02:03`, or `4:05` under an hour.
    pub duration_label: String,
    /// The day it was streamed or published (`Oct 24`); `None` when it
    /// cannot be shown.
    pub date_label: Option<String>,
    pub thumbnail_url: String,
    /// The category it was in; `None`: none shown.
    pub game_name: Option<String>,
    /// How far in playback would resume, in (0, 1), for a bar over the
    /// thumbnail; `None`: no bar.
    pub progress: Option<f64>,
}
