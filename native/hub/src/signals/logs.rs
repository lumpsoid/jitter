use rinf::{DartSignal, RustSignal, SignalPiece};
use serde::{Deserialize, Serialize};

/// The logs page opened: answer with `Sessions`.
#[derive(Deserialize, DartSignal)]
pub struct LogsReady;

/// The user picked session `name` from the list: answer with `Opened`.
#[derive(Deserialize, DartSignal)]
pub struct OpenLog {
    pub name: String,
}

/// A line for this session's log from Dart: an error it caught, or one
/// mpv reported.
#[derive(Deserialize, DartSignal)]
pub struct LogFromDart {
    pub level: DartLogLevel,
    /// Where it came from: `flutter`, `mpv/ffmpeg`.
    pub source: String,
    pub message: String,
}

/// How bad a [`LogFromDart`] line is.
#[derive(Deserialize, SignalPiece)]
pub enum DartLogLevel {
    Error,
    Warning,
    Info,
    /// Only the video's lines are kept: mpv's decoder and renderer, with
    /// the GL context it names there; the rest would drown the log.
    Verbose,
}

/// The player started or stopped waiting for data. Only logged: how long
/// each wait took, and how much it had buffered ahead when it began.
#[derive(Deserialize, DartSignal)]
pub struct PlayerBuffering {
    pub buffering: bool,
    pub position_ms: u32,
    /// How far it has buffered, on the same clock as `position_ms`.
    pub buffer_ms: u32,
}

/// The screens Flutter sees, once at startup. Only logged.
#[derive(Deserialize, DartSignal)]
pub struct DisplaysProbe {
    pub displays: Vec<DisplayFacts>,
}

/// One screen of a [`DisplaysProbe`].
#[derive(Deserialize, SignalPiece)]
pub struct DisplayFacts {
    /// As Flutter reports it: physical pixels, except on Linux under
    /// Wayland fractional scaling, where GTK gives the logical size. Each
    /// value is Flutter's own, unchecked: Android's embedder gives `-1`
    /// for what it does not know.
    pub width: f64,
    pub height: f64,
    pub refresh_hz: f64,
    /// Logical pixels to physical.
    pub scale: f64,
    /// The app's window is on it.
    pub has_window: bool,
}

/// The player's libmpv and FFmpeg, once it is created. Only logged. Each
/// is mpv's own text, empty where mpv did not say.
#[derive(Deserialize, DartSignal)]
pub struct MpvVersions {
    pub mpv: String,
    pub ffmpeg: String,
}

/// How mpv plays what it just loaded. Only logged. Each is mpv's own
/// property text, empty where mpv did not say.
#[derive(Deserialize, DartSignal)]
pub struct PlaybackProbe {
    /// `hwdec-current`: `no` when decoding in software.
    pub hwdec: String,
    pub hwdec_interop: String,
    pub vo: String,
    pub gpu_context: String,
    pub video_codec: String,
    pub width: String,
    pub height: String,
    pub fps: String,
    pub pixelformat: String,
    /// The surface format a hardware decoder hands over.
    pub hw_pixelformat: String,
    pub audio_codec: String,
    pub ao: String,
}

/// The frames mpv lost over what it played, as it stops or switches.
/// Only logged. Each is mpv's own count, empty where mpv did not say.
#[derive(Deserialize, DartSignal)]
pub struct PlaybackDrops {
    /// `frame-drop-count`: dropped by the video output.
    pub output: String,
    /// `decoder-frame-drop-count`.
    pub decoder: String,
    /// `mistimed-frame-count`.
    pub mistimed: String,
}

/// The logs page: the session list, then the session the user opened.
#[derive(Serialize, RustSignal)]
pub enum LogsUpdate {
    /// Every kept session, newest first, and the directory they are in.
    Sessions {
        dir: String,
        sessions: Vec<LogSession>,
    },
    /// Session `name`'s whole log, and its file to share.
    Opened {
        name: String,
        path: String,
        text: String,
    },
    /// The list or a session could not be read. A one-shot notice.
    Failed { message: String },
}

/// One kept session: `name` is its file's (`2026-11-03_14-22-05.log`).
#[derive(Serialize, SignalPiece)]
pub struct LogSession {
    pub name: String,
    /// When it started, to show: `2026-11-03 14:22:05`.
    pub label: String,
    /// The session running now.
    pub current: bool,
}
