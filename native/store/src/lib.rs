//! SQLite persistence: implements core's storage ports. No decisions.
//!
//! One database file is shared by every head on the machine (the GUI and any
//! number of CLI processes at once), so the store is written for concurrent
//! connections: WAL journal, migrations under an `IMMEDIATE` transaction, and
//! no upsert that could bring back a row another process deleted.

mod favorite_rows;
mod history_rows;
mod migrations;
mod resume_rows;
mod settings_rows;
mod sqlite_store;

use std::fs;
use std::path::{Path, PathBuf};

pub use jitter_core::ports::StoreError;
pub use sqlite_store::SqliteStore;

const APP_DIR: &str = "jitter";
const DB_FILE: &str = "jitter.db";

/// The one database every desktop head opens: `<data dir>/jitter/jitter.db`
/// (Linux `$XDG_DATA_HOME` or `~/.local/share`, macOS
/// `~/Library/Application Support`, Windows `%APPDATA%`). Creates the
/// `jitter` directory if needed.
///
/// Defined here, in Rust, so the CLI and the desktop GUI cannot disagree on
/// the file. Mobile heads have no shared location and open the directory
/// their platform gives them instead.
pub fn default_db_path() -> Result<PathBuf, StoreError> {
    let base = dirs::data_dir()
        .ok_or_else(|| StoreError::new("no data directory for this user and platform"))?;
    db_path_under(&base)
}

/// `<base>/jitter/jitter.db`, creating `<base>/jitter`.
fn db_path_under(base: &Path) -> Result<PathBuf, StoreError> {
    let dir = base.join(APP_DIR);
    fs::create_dir_all(&dir)
        .map_err(|e| StoreError::new(format!("cannot create {}: {e}", dir.display())))?;
    Ok(dir.join(DB_FILE))
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn path_is_jitter_db_in_a_created_jitter_dir() -> TestResult {
        let base = tempfile::tempdir()?;
        let path = db_path_under(base.path())?;
        assert_eq!(path, base.path().join("jitter").join("jitter.db"));
        assert!(base.path().join("jitter").is_dir());
        Ok(())
    }

    /// Calls the real function, so it creates `<data dir>/jitter` as any
    /// head would on first run.
    #[test]
    fn default_path_is_under_the_platform_data_dir() -> TestResult {
        let base = dirs::data_dir().ok_or("no data dir")?;
        assert_eq!(default_db_path()?, base.join("jitter").join("jitter.db"));
        Ok(())
    }
}
