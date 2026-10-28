use rinf::{DartSignal, RustSignal, SignalPiece};
use serde::{Deserialize, Serialize};

use super::{PlaybackChannel, PlaybackTarget};

/// The history holder is subscribed: answer with a `Snapshot`.
#[derive(Deserialize, DartSignal)]
pub struct HistoryReady;

/// Remove the one visit `id` from the timeline.
#[derive(Deserialize, DartSignal)]
pub struct RemoveVisit {
    pub id: i64,
}

/// Remove every visit to `target`: an entry of the unique list.
#[derive(Deserialize, DartSignal)]
pub struct RemoveFromHistory {
    pub target: PlaybackTarget,
}

/// Remove every visit.
#[derive(Deserialize, DartSignal)]
pub struct ClearHistory;

/// The history: a snapshot after every change to the list. One type, so a
/// snapshot and the failures around it arrive in order.
#[derive(Serialize, RustSignal)]
pub enum HistoryUpdate {
    /// Replaces the history.
    Snapshot {
        /// The timeline: every visit, by the day it started, newest day
        /// and newest visit first.
        days: Vec<HistoryDay>,
        /// The latest visit to each live channel or VOD, newest first.
        unique: Vec<HistoryEntry>,
    },
    /// A removal failed; the history is as last sent. A one-shot notice
    /// for a snackbar, not history state.
    Failed { message: String },
}

/// One day of the timeline: `label` is `Today`, `Yesterday` or the date
/// (`Mon, Oct 24`).
#[derive(Serialize, SignalPiece)]
pub struct HistoryDay {
    pub label: String,
    pub entries: Vec<HistoryEntry>,
}

/// A visit as its row shows it, as it was when the visit started.
/// Tapping it plays `target` again; `id` goes back in [`RemoveVisit`],
/// `target` in [`RemoveFromHistory`].
#[derive(Serialize, SignalPiece)]
pub struct HistoryEntry {
    pub id: i64,
    pub target: PlaybackTarget,
    pub live: bool,
    /// The stream's or the VOD's title, else the channel's name.
    pub title: String,
    pub channel: Option<PlaybackChannel>,
    pub game: Option<String>,
    /// When it started, ready to show: the time (`14:02`) on the
    /// timeline, the day and time (`Yesterday, 14:02`) in the unique list.
    pub when: String,
    /// How long it was watched: `<1m`, `40m`, `1h 5m`.
    pub watched: String,
    /// The day a VOD was streamed (`Oct 24`); `None` for a live stream or
    /// when it is not known.
    pub streamed: Option<String>,
}
