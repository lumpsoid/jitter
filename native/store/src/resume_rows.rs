use jitter_core::models::{ResumePoint, Seconds, Timestamp, VideoId};
use jitter_core::ports::{ResumeStore, StoreError};
use rusqlite::{Connection, OptionalExtension};

use crate::sqlite_store::{SqliteStore, bad_row, db_error};

/// Last writer wins: `put` replaces the point for its video, which is the
/// right answer for "where did I stop" when two heads played the same VOD.
impl ResumeStore for SqliteStore {
    fn get(&self, id: &VideoId) -> Result<Option<ResumePoint>, StoreError> {
        let row: Option<(i64, i64, i64)> = self
            .conn()?
            .query_row(
                "SELECT position_s, duration_s, saved_at FROM resume_points WHERE video_id = ?1",
                [id.as_str()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(db_error)?;
        row.map(|(position, duration, saved_at)| {
            Ok(ResumePoint {
                video_id: id.clone(),
                position: seconds("position_s", position)?,
                duration: seconds("duration_s", duration)?,
                saved_at: Timestamp::from_unix(saved_at),
            })
        })
        .transpose()
    }

    fn put(&mut self, point: &ResumePoint) -> Result<(), StoreError> {
        self.conn()?
            .execute(
                "INSERT INTO resume_points (video_id, position_s, duration_s, saved_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (video_id) DO UPDATE SET
                     position_s = excluded.position_s,
                     duration_s = excluded.duration_s,
                     saved_at   = excluded.saved_at",
                (
                    point.video_id.as_str(),
                    point.position.get(),
                    point.duration.get(),
                    point.saved_at.unix(),
                ),
            )
            .map(drop)
            .map_err(db_error)
    }

    fn clear(&mut self, id: &VideoId) -> Result<(), StoreError> {
        self.conn()?
            .execute(
                "DELETE FROM resume_points WHERE video_id = ?1",
                [id.as_str()],
            )
            .map(drop)
            .map_err(db_error)
    }
}

/// Every stored point, by video id.
pub(crate) fn all(conn: &Connection) -> Result<Vec<ResumePoint>, StoreError> {
    let rows: Vec<(String, i64, i64, i64)> = conn
        .prepare(
            "SELECT video_id, position_s, duration_s, saved_at
             FROM resume_points ORDER BY video_id",
        )
        .and_then(|mut stmt| {
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
                .collect()
        })
        .map_err(db_error)?;
    rows.into_iter()
        .map(|(id, position, duration, saved_at)| {
            Ok(ResumePoint {
                video_id: VideoId::parse(&id)
                    .map_err(|_| bad_row("resume_points", "video_id", &id))?,
                position: seconds("position_s", position)?,
                duration: seconds("duration_s", duration)?,
                saved_at: Timestamp::from_unix(saved_at),
            })
        })
        .collect()
}

/// Stores `point` unless the point stored for its video was saved at the
/// same time or later. Returns the rows written, 0 or 1.
pub(crate) fn put_if_newer(conn: &Connection, point: &ResumePoint) -> rusqlite::Result<usize> {
    conn.execute(
        "INSERT INTO resume_points (video_id, position_s, duration_s, saved_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (video_id) DO UPDATE SET
             position_s = excluded.position_s,
             duration_s = excluded.duration_s,
             saved_at   = excluded.saved_at
         WHERE excluded.saved_at > resume_points.saved_at",
        (
            point.video_id.as_str(),
            point.position.get(),
            point.duration.get(),
            point.saved_at.unix(),
        ),
    )
}

fn seconds(column: &str, value: i64) -> Result<Seconds, StoreError> {
    u32::try_from(value)
        .map(Seconds::new)
        .map_err(|_| bad_row("resume_points", column, value))
}
