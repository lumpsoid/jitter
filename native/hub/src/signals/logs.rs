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
