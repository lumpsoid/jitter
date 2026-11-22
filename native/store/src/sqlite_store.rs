use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use jitter_core::ports::StoreError;
use rusqlite::{Connection, ErrorCode};

use crate::migrations;

/// One migrated SQLite connection that implements `DownloadsStore`,
/// `FavoritesStore`, `HistoryStore`, `ResumeStore` and `SettingsStore`. Clones share the
/// connection, so a head hands one clone to each controller; an in-memory
/// database stays a single database.
///
/// Other processes may have the same file open. Rows are never cached here:
/// every call reads or writes the database.
#[derive(Clone)]
pub struct SqliteStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteStore {
    /// Opens (creating if needed) the database file at `path`, switches it
    /// to WAL so readers and a writer in other processes do not block each
    /// other, and brings its schema up to date. Commits are not fsynced
    /// (`synchronous=NORMAL`): see `sync_at_checkpoints`.
    ///
    /// Opening waits up to `OPEN_WAIT` for another connection's lock, and
    /// every later call up to `CALL_WAIT`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let mut conn = Connection::open(path).map_err(db_error)?;
        conn.busy_timeout(OPEN_WAIT).map_err(db_error)?;
        enable_wal(&conn)?;
        sync_at_checkpoints(&conn)?;
        migrations::migrate(&mut conn)?;
        conn.busy_timeout(CALL_WAIT).map_err(db_error)?;
        Ok(Self::new(conn))
    }

    /// A fresh, migrated database that lives as long as this store and its
    /// clones. In-memory databases have no WAL; nothing else shares them.
    pub fn open_in_memory() -> Result<Self, StoreError> {
        let mut conn = Connection::open_in_memory().map_err(db_error)?;
        migrations::migrate(&mut conn)?;
        Ok(Self::new(conn))
    }

    fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    pub(crate) fn conn(&self) -> Result<MutexGuard<'_, Connection>, StoreError> {
        self.conn
            .lock()
            .map_err(|_| StoreError::new("database connection lock poisoned"))
    }
}

/// How long a call waits for another connection's lock: rusqlite's
/// default busy timeout. It covers another process's write.
const CALL_WAIT: Duration = Duration::from_secs(5);

/// How long `open` waits for another connection's lock, and keeps retrying
/// the switch to WAL. Longer than `CALL_WAIT`: another head opening the
/// same file holds its lock through a migration's commit, or through the
/// checkpoint when it closes, and both wait on fsync. On a busy SD card
/// that took more than 5 s, and the second head failed to open with
/// "database is locked".
const OPEN_WAIT: Duration = Duration::from_secs(30);
const WAL_SWITCH_RETRY: Duration = Duration::from_millis(10);

/// Puts the file in WAL mode. The switch reads the header and then upgrades
/// to a write lock, and SQLite does not run the busy handler for that
/// upgrade: when two connections switch a fresh file at once, one gets
/// `SQLITE_BUSY` immediately. The other's switch then completes, so the
/// loser retries and finds the file already in WAL.
fn enable_wal(conn: &Connection) -> Result<(), StoreError> {
    let deadline = Instant::now() + OPEN_WAIT;
    loop {
        match conn
            .pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get::<_, String>(0))
        {
            Ok(mode) if mode.eq_ignore_ascii_case("wal") => return Ok(()),
            Ok(mode) => {
                return Err(StoreError::new(format!(
                    "database refused WAL journal mode (got {mode})"
                )));
            }
            Err(e) if is_busy(&e) && Instant::now() < deadline => {
                thread::sleep(WAL_SWITCH_RETRY);
            }
            Err(e) => return Err(db_error(e)),
        }
    }
}

/// Sets `synchronous=NORMAL`: in WAL, SQLite then fsyncs only at a
/// checkpoint, not at every commit. The file stays consistent; a power cut
/// can lose the last commits (a resume point, a visit, a favorite added just
/// before), an app crash loses nothing. Each fsync on a busy SD card held
/// another head's open waiting for seconds. The setting is per connection.
fn sync_at_checkpoints(conn: &Connection) -> Result<(), StoreError> {
    conn.pragma_update(None, "synchronous", "NORMAL")
        .map_err(db_error)
}

fn is_busy(error: &rusqlite::Error) -> bool {
    error.sqlite_error_code() == Some(ErrorCode::DatabaseBusy)
}

pub(crate) fn db_error(error: rusqlite::Error) -> StoreError {
    StoreError::new(error.to_string())
}

/// A stored value that core's value objects reject: the row is corrupt or
/// was written by something other than this store.
pub(crate) fn bad_row(table: &str, column: &str, value: impl std::fmt::Display) -> StoreError {
    StoreError::new(format!("invalid {table}.{column} in database: {value}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// `PRAGMA synchronous` reads 1 for NORMAL (2 is FULL, the default).
    #[test]
    fn an_opened_file_syncs_only_at_checkpoints() -> TestResult {
        let dir = tempfile::tempdir()?;
        let store = SqliteStore::open(dir.path().join("jitter.db"))?;
        let synchronous: i64 = store
            .conn()?
            .pragma_query_value(None, "synchronous", |row| row.get(0))?;
        assert_eq!(synchronous, 1);
        Ok(())
    }
}
