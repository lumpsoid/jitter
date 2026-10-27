use rinf::{DartSignal, RustSignal, SignalPiece};
use serde::{Deserialize, Serialize};

/// What to play: a channel live, or a VOD. Dart holds it for the page's
/// lifetime and tags every report with it, so Rust can tell a page that
/// was replaced from the one showing now.
#[derive(Deserialize, SignalPiece)]
pub enum PlaybackTarget {
    Live { login: String },
    Vod { id: String },
}

/// The player page opened: resolve `target` and play it.
#[derive(Deserialize, DartSignal)]
pub struct Play {
    pub target: PlaybackTarget,
}

/// The user picked a quality from the menu. `position_s` is where the
/// player is now, so the new quality carries on from there.
#[derive(Deserialize, DartSignal)]
pub struct SelectQuality {
    pub name: String,
    pub position_s: u32,
}

/// Where the player showing `target` is. Sent about every second and once
/// more with [`LeavePlayer`]; Rust decides whether it is worth saving.
/// `duration_s` is 0 until the player has loaded.
#[derive(Deserialize, DartSignal)]
pub struct PlayerPosition {
    pub target: PlaybackTarget,
    pub position_s: u32,
    pub duration_s: u32,
}

/// The player page for `target` closed, with the player's last position.
#[derive(Deserialize, DartSignal)]
pub struct LeavePlayer {
    pub target: PlaybackTarget,
    pub position_s: u32,
    pub duration_s: u32,
}

/// The player page's view state; Dart replaces what it has.
#[derive(Serialize, RustSignal)]
pub enum PlaybackUpdate {
    Idle,
    Resolving,
    /// `live` streams have no seek bar. `selected` is one of the
    /// `qualities`' names.
    Ready {
        qualities: Vec<QualityOption>,
        selected: String,
        live: bool,
    },
    Failed {
        message: String,
    },
}

/// One entry of the quality menu: `name` goes back in [`SelectQuality`],
/// `label` is shown.
#[derive(Serialize, SignalPiece)]
pub struct QualityOption {
    pub name: String,
    pub label: String,
}

/// A command for the video player, a platform effect: Dart performs it and
/// keeps nothing.
#[derive(Serialize, RustSignal)]
pub enum PlayerCommand {
    Open { url: String, start_s: u32 },
    Stop,
}
