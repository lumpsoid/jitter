//! Backups: backup signals ↔ the store. Dart picks the files; Rust writes
//! and reads their text. After an import `run.rs` reloads every feature
//! the backup reaches.

use jitter_store::{Imported, SqliteStore};
use rinf::RustSignal;

use crate::format::today;
use crate::playback::now;
use crate::signals::{BackupUpdate, ImportBackup};

/// `ExportBackup`: everything stored, or the failure.
pub fn export(store: &SqliteStore) {
    let exported = now().and_then(|now| store.export_backup(now).map_err(|e| e.to_string()));
    match exported {
        Ok(json) => BackupUpdate::Exported {
            file_name: format!("jitter-backup-{}.json", today().format("%Y-%m-%d")),
            json,
        },
        Err(e) => BackupUpdate::Failed {
            message: format!("Couldn't export: {e}"),
        },
    }
    .send_signal_to_dart();
}

/// `ImportBackup`: merges the backup in and sends what it added, or the
/// failure. Returns whether it was imported.
pub fn import(store: &mut SqliteStore, signal: &ImportBackup) -> bool {
    match store.import_backup(&signal.json) {
        Ok(imported) => {
            BackupUpdate::Imported {
                summary: summary(imported),
            }
            .send_signal_to_dart();
            true
        }
        Err(e) => {
            BackupUpdate::Failed {
                message: format!("Couldn't import: {e}"),
            }
            .send_signal_to_dart();
            false
        }
    }
}

/// `Imported 1 favorite, 12 visits and 0 resume points.`
fn summary(imported: Imported) -> String {
    format!(
        "Imported {}, {} and {}.",
        count(imported.favorites, "favorite"),
        count(imported.visits, "visit"),
        count(imported.resume_points, "resume point"),
    )
}

fn count(n: usize, noun: &str) -> String {
    match n {
        1 => format!("1 {noun}"),
        n => format!("{n} {noun}s"),
    }
}
