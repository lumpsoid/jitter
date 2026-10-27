use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use jitter_core::ports::StoreError;
use rusqlite::{Connection, ErrorCode};

use crate::migrations;

/// One migrated SQLite connection that implements `FavoritesStore`,
/// `ResumeStore` and `SettingsStore`. Clones share the connection, so a head hands one clone to
/// each controller; an in-memory database stays a single database.
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
    /// other, and brings its schema up to date.
    ///
    /// rusqlite's default 5 s busy timeout is kept: it covers waiting for
    /// another process's write to finish.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let conn = Connection::open(path).map_err(db_error)?;
        enable_wal(&conn)?;
        Self::ready(conn)
    }

    /// A fresh, migrated database that lives as long as this store and its
    /// clones. In-memory databases have no WAL; nothing else shares them.
    pub fn open_in_memory() -> Result<Self, StoreError> {
        Self::ready(Connection::open_in_memory().map_err(db_error)?)
    }

    fn ready(mut conn: Connection) -> Result<Self, StoreError> {
        migrations::migrate(&mut conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub(crate) fn conn(&self) -> Result<MutexGuard<'_, Connection>, StoreError> {
        self.conn
            .lock()
            .map_err(|_| StoreError::new("database connection lock poisoned"))
    }
}

/// How long `open` keeps retrying the switch to WAL while another
/// connection is switching the same file; rusqlite's busy timeout.
const WAL_SWITCH_WAIT: Duration = Duration::from_secs(5);
const WAL_SWITCH_RETRY: Duration = Duration::from_millis(10);

/// Puts the file in WAL mode. The switch reads the header and then upgrades
/// to a write lock, and SQLite does not run the busy handler for that
/// upgrade: when two connections switch a fresh file at once, one gets
/// `SQLITE_BUSY` immediately. The other's switch then completes, so the
/// loser retries and finds the file already in WAL.
fn enable_wal(conn: &Connection) -> Result<(), StoreError> {
    let deadline = Instant::now() + WAL_SWITCH_WAIT;
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
