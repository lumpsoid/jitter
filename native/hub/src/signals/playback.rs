use rinf::{DartSignal, RustSignal, SignalPiece};
use serde::{Deserialize, Serialize};

/// What to play: a channel live, or a VOD. Dart holds it for the page's
/// lifetime and tags every report with it, so Rust can tell a page that
/// was replaced from the one showing now. Rust sends it in a history
/// entry, to play again.
#[derive(Clone, Serialize, Deserialize, SignalPiece)]
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

/// The user typed a time to go to in the VOD playing, `duration_s` long
/// (0 until the player has loaded). Answered with [`GoToTimeAnswer`] and,
/// when it is a time, a [`PlayerCommand::Seek`].
#[derive(Deserialize, DartSignal)]
pub struct GoToTime {
    pub text: String,
    pub duration_s: u32,
}

/// The user picked chapter `index` of the VOD playing, one of
/// [`PlaybackDetails`]'s `chapters`: Rust answers with a
/// [`PlayerCommand::Seek`] to its start.
#[derive(Deserialize, DartSignal)]
pub struct GoToChapter {
    pub index: u32,
}

/// The player page for `target` closed, with the player's last position.
#[derive(Deserialize, DartSignal)]
pub struct LeavePlayer {
    pub target: PlaybackTarget,
    pub position_s: u32,
    pub duration_s: u32,
}

/// The user armed the player page's sleep timer: stop playing `minutes`
/// from now (one of [`PlaybackSleep`]'s `presets`), replacing a timer
/// already armed.
#[derive(Deserialize, DartSignal)]
pub struct ArmSleepTimer {
    pub minutes: u32,
}

/// The user turned the player page's sleep timer off.
#[derive(Deserialize, DartSignal)]
pub struct TurnOffSleepTimer;

/// The app went to the background, with the player at `position_s`. Rust
/// decides whether the player carries on with audio only or stops.
#[derive(Deserialize, DartSignal)]
pub struct AppBackgrounded {
    pub position_s: u32,
}

/// The app is back in front, with the player at `position_s`.
#[derive(Deserialize, DartSignal)]
pub struct AppForegrounded {
    pub position_s: u32,
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
        /// Boxed: it would make this variant far larger than the others.
        details: Box<PlaybackDetails>,
        sleep: PlaybackSleep,
    },
    /// The sleep timer stopped the player; the page offers to resume,
    /// which plays the same target again.
    Slept,
    Failed {
        message: String,
    },
}

/// The answer to [`GoToTime`], once each, for the box the time was typed
/// in: not view state, so apart from [`PlaybackUpdate`]. Nothing else
/// waits on its order.
#[derive(Serialize, RustSignal)]
pub enum GoToTimeAnswer {
    /// The player seeks there.
    Went,
    /// Not a time in this VOD; `message` says why, ready to show.
    Rejected { message: String },
}

/// The player page's sleep timer menu.
#[derive(Serialize, SignalPiece)]
pub struct PlaybackSleep {
    /// The timers it offers; each one's `minutes` goes back in
    /// [`ArmSleepTimer`].
    pub presets: Vec<SleepPreset>,
    /// While armed, the time left, ready to show (`23 min left`); `None`
    /// while no timer is armed.
    pub left: Option<String>,
}

/// One sleep timer the menu offers: `label` is shown (`15 minutes`).
#[derive(Serialize, SignalPiece)]
pub struct SleepPreset {
    pub minutes: u32,
    pub label: String,
}

/// What is playing, shown on the player page beside the video.
#[derive(Serialize, SignalPiece)]
pub struct PlaybackDetails {
    /// The channel being watched (a VOD's owner); `None` when it is not
    /// known.
    pub channel: Option<PlaybackChannel>,
    /// The stream's or the VOD's title; `None` when there is none to show.
    pub title: Option<String>,
    /// The game or category's name: for a VOD with chapters, the one at
    /// the player's position, sent again when it changes. `None` when
    /// there is none to show.
    pub game: Option<String>,
    /// A VOD's chapters to go to, in the order they start; empty for a
    /// live stream or a VOD in one chapter.
    pub chapters: Vec<PlaybackChapter>,
    /// The day a VOD was streamed, ready to show (`Oct 24`); `None` for a
    /// live stream or when it is not known.
    pub streamed: Option<String>,
    /// A live stream's figures; `None` for a VOD or when they are not
    /// known.
    pub live: Option<PlaybackLive>,
}

/// One chapter of a VOD, ready to show: going to it sends its place in
/// [`PlaybackDetails`]'s `chapters` as [`GoToChapter`]'s `index`.
#[derive(Serialize, SignalPiece)]
pub struct PlaybackChapter {
    /// The game played in it.
    pub game: String,
    /// Where it starts (`1:02:03`, or `4:05` under an hour).
    pub start: String,
    /// Whether it is the one playing now.
    pub current: bool,
}

/// What a live stream shows beside its video, ready to show.
#[derive(Serialize, SignalPiece)]
pub struct PlaybackLive {
    /// `1.2K viewers`.
    pub viewers: String,
    /// How long it has been live as of when the state was sent (`2h 15m`);
    /// `None` when its start or the clock is not known.
    pub uptime: Option<String>,
}

/// The channel a player page shows. Tapping it opens the channel by
/// `login`.
#[derive(Serialize, SignalPiece)]
pub struct PlaybackChannel {
    pub login: String,
    pub display_name: String,
    /// `None`: no avatar known, show a placeholder.
    pub avatar_url: Option<String>,
}

/// One entry of the quality menu: `name` goes back in [`SelectQuality`],
/// `label` is shown.
#[derive(Serialize, SignalPiece)]
pub struct QualityOption {
    pub name: String,
    pub label: String,
}

/// A command for the video player, a platform effect: Dart performs it and
/// keeps nothing but the user's own volume under a fade.
#[derive(Serialize, RustSignal)]
pub enum PlayerCommand {
    Open {
        url: String,
        start_s: u32,
    },
    Stop,
    /// Go to `position_s` in the VOD playing.
    Seek {
        position_s: u32,
    },
    /// Play at `level` (0.0 silence – 1.0) times the user's own volume,
    /// which stays the user's: the sleep timer's fade-out. An `Open` or a
    /// `Stop` ends the fade, as does a `level` of 1.0.
    SetFade {
        level: f64,
    },
}
