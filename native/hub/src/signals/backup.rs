use rinf::{DartSignal, RustSignal};
use serde::{Deserialize, Serialize};

/// The user asked for a backup: Rust answers `Exported` with it.
#[derive(Deserialize, DartSignal)]
pub struct ExportBackup;

/// The text of a backup file the user picked. Rust merges it into what is
/// stored and answers `Imported`; the favorites, the history and the
/// settings go out again.
#[derive(Deserialize, DartSignal)]
pub struct ImportBackup {
    pub json: String,
}

/// A backup's answer: one-shot notices, not state.
#[derive(Serialize, RustSignal)]
pub enum BackupUpdate {
    /// The backup, for Dart to save under `file_name`
    /// (`jitter-backup-2026-11-10.json`) where the user chooses.
    Exported { file_name: String, json: String },
    /// What the import added, to show the user: `Imported 3 favorites, …`.
    Imported { summary: String },
    /// An export or import failed; an import that failed changed nothing.
    Failed { message: String },
}
