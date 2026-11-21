use rinf::{DartSignal, RustSignal, SignalPiece};
use serde::{Deserialize, Serialize};

use super::QualityOption;

/// The downloads list is subscribed: answer with a `Snapshot`.
#[derive(Deserialize, DartSignal)]
pub struct DownloadsReady;

/// The user asked for a VOD's qualities before choosing one: Rust answers
/// with `Choices`, or `Failed` when they cannot be loaded.
#[derive(Deserialize, DartSignal)]
pub struct ChooseDownload {
    /// The VOD's id, as the list shows it.
    pub video_id: String,
}

/// The user chose a quality and a directory: Rust starts the download and
/// answers with a `Snapshot` that lists it, or with `Failed`.
#[derive(Deserialize, DartSignal)]
pub struct StartDownload {
    pub video_id: String,
    /// A quality's name, as `Choices` offered it.
    pub quality: String,
    /// The directory the file is written into; chosen by Dart.
    pub dir: String,
}

/// The user cancelled a running download: Rust stops it, deletes its
/// partial file and answers with a `Snapshot`.
#[derive(Deserialize, DartSignal)]
pub struct CancelDownload {
    pub key: u64,
}

/// The user removed a finished, failed or cancelled download from the
/// list: Rust answers with a `Snapshot`.
#[derive(Deserialize, DartSignal)]
pub struct RemoveDownload {
    pub key: u64,
}

/// What the downloads feature tells Dart. `Snapshot` replaces the list;
/// `Choices` and `Failed` are one-shot, not state.
#[derive(Serialize, RustSignal)]
pub enum DownloadsUpdate {
    /// Every download, oldest first. Replaces the list.
    Snapshot { downloads: Vec<DownloadItem> },
    /// A VOD's qualities, for the user to choose one. One-shot.
    Choices {
        video_id: String,
        qualities: Vec<QualityOption>,
    },
    /// A request for `video_id` failed, or its download failed to start.
    /// One-shot notice; tagged so a flow waiting on one VOD skips another's.
    Failed { video_id: String, message: String },
}

/// One download in the list.
#[derive(Serialize, SignalPiece)]
pub struct DownloadItem {
    /// Names the download in `CancelDownload` and `RemoveDownload`.
    pub key: u64,
    pub video_id: String,
    /// The file being written, `<login>_<id>_<quality>.<ext>`.
    pub file_name: String,
    /// Segments written so far, of `total`. Counts past `u32::MAX` read as it.
    pub done: u32,
    pub total: u32,
    /// Bytes written so far.
    pub bytes: u64,
    pub state: DownloadState,
}

/// Where a download stands.
#[derive(Serialize, SignalPiece)]
pub enum DownloadState {
    Running,
    /// The final file's full path.
    Done {
        path: String,
    },
    /// Why it stopped; its partial file is kept.
    Failed {
        message: String,
    },
    /// Stopped by the user; its partial file is deleted.
    Cancelled,
}
