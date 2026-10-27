//! Schema migrations keyed by `PRAGMA user_version`. Each entry takes the
//! database to its version from the one before; add a version by appending
//! one entry. A shipped entry is never edited.

use jitter_core::ports::StoreError;
use rusqlite::{Connection, TransactionBehavior};

use crate::sqlite_store::db_error;

/// One schema version: `sql` takes the database to `version`.
struct Migration {
    version: u32,
    sql: &'static str,
}

/// In ascending version order starting at 1.
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: V1_SCHEMA,
    },
    Migration {
        version: 2,
        sql: V2_FAVORITE_AVATARS,
    },
];

/// v1: favorite channels (login and display name are display caches; the
/// key is the channel id) and VOD resume points. Times are Unix seconds,
/// positions and durations whole seconds.
const V1_SCHEMA: &str = "
CREATE TABLE favorites (
    channel_id   TEXT PRIMARY KEY NOT NULL,
    login        TEXT NOT NULL,
    display_name TEXT NOT NULL,
    added_at     INTEGER NOT NULL
);

CREATE TABLE resume_points (
    video_id   TEXT PRIMARY KEY NOT NULL,
    position_s INTEGER NOT NULL,
    duration_s INTEGER NOT NULL,
    saved_at   INTEGER NOT NULL
);
";

/// v2: a favorite's avatar URL, another display cache. Nullable: rows from
/// v1 have none until the channel is next seen live.
const V2_FAVORITE_AVATARS: &str = "
ALTER TABLE favorites ADD COLUMN avatar_url TEXT;
";

/// Applies every pending migration in one transaction: the database ends at
/// the latest version or is left as it was. A database newer than this build
/// is refused rather than written with an older schema's assumptions.
///
/// The transaction is `IMMEDIATE` and `user_version` is read inside it.
/// Several processes may open a fresh file at once; under a `DEFERRED`
/// transaction each would take a read lock and then fail to upgrade it with
/// `SQLITE_BUSY` straight away (the busy timeout does not apply to that
/// deadlock). `IMMEDIATE` takes the write lock first, so the others wait,
/// then read the version the winner wrote and find nothing to do.
pub(crate) fn migrate(conn: &mut Connection) -> Result<(), StoreError> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(db_error)?;
    let current: u32 = tx
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(db_error)?;
    let latest = MIGRATIONS.last().map_or(0, |m| m.version);
    if current > latest {
        return Err(StoreError::new(format!(
            "database schema v{current} is newer than this app (v{latest})"
        )));
    }
    for m in MIGRATIONS.iter().filter(|m| m.version > current) {
        tx.execute_batch(m.sql).map_err(db_error)?;
        tx.pragma_update(None, "user_version", m.version)
            .map_err(db_error)?;
    }
    tx.commit().map_err(db_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn user_version(conn: &Connection) -> rusqlite::Result<u32> {
        conn.pragma_query_value(None, "user_version", |row| row.get(0))
    }

    fn tables(conn: &Connection) -> rusqlite::Result<Vec<String>> {
        conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")?
            .query_map([], |r| r.get(0))?
            .collect()
    }

    #[test]
    fn empty_database_ends_at_v2_with_both_tables() -> TestResult {
        let mut conn = Connection::open_in_memory()?;
        migrate(&mut conn)?;
        assert_eq!(user_version(&conn)?, 2);
        assert_eq!(tables(&conn)?, ["favorites", "resume_points"]);
        Ok(())
    }

    #[test]
    fn migrating_a_migrated_database_changes_nothing() -> TestResult {
        let mut conn = Connection::open_in_memory()?;
        migrate(&mut conn)?;
        conn.execute("INSERT INTO favorites VALUES ('1', 'a', 'A', 0, NULL)", [])?;
        migrate(&mut conn)?;
        assert_eq!(user_version(&conn)?, 2);
        let rows: i64 = conn.query_row("SELECT COUNT(*) FROM favorites", [], |r| r.get(0))?;
        assert_eq!(rows, 1);
        Ok(())
    }

    #[test]
    fn a_v1_database_gains_a_null_avatar_and_keeps_its_rows() -> TestResult {
        let mut conn = Connection::open_in_memory()?;
        conn.execute_batch(V1_SCHEMA)?;
        conn.pragma_update(None, "user_version", 1)?;
        conn.execute("INSERT INTO favorites VALUES ('1', 'a', 'A', 7)", [])?;
        migrate(&mut conn)?;
        assert_eq!(user_version(&conn)?, 2);
        let row: (String, Option<String>, i64) = conn.query_row(
            "SELECT login, avatar_url, added_at FROM favorites WHERE channel_id = '1'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        assert_eq!(row, ("a".to_owned(), None, 7));
        Ok(())
    }

    #[test]
    fn newer_database_is_refused_and_left_alone() -> TestResult {
        let mut conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "user_version", 3)?;
        assert!(migrate(&mut conn).is_err());
        assert_eq!(user_version(&conn)?, 3);
        assert!(tables(&conn)?.is_empty());
        Ok(())
    }
}
