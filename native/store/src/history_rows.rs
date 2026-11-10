use jitter_core::models::{
    CategoryId, Channel, ChannelId, ChannelLogin, GameRef, NewVisit, Timestamp, VideoId, Visit,
    VisitDetails, VisitId,
};
use jitter_core::playback::PlaybackTarget;
use jitter_core::ports::{HistoryStore, StoreError};
use rusqlite::{Connection, OptionalExtension, Row};

use crate::sqlite_store::{SqliteStore, bad_row, db_error};

const COLUMNS: &str = "id, kind, target, started_at, last_seen_at, title, channel_id, \
                       channel_login, channel_name, channel_avatar, game_id, game_name, \
                       streamed_at";

const NEWEST_FIRST: &str = "ORDER BY started_at DESC, id DESC";

/// The columns a visit is written with, as `?1` to `?12` in [`insert`].
const WRITTEN: &str = "kind, target, started_at, last_seen_at, title, channel_id, \
                       channel_login, channel_name, channel_avatar, game_id, game_name, \
                       streamed_at";

const VALUES: &str = "?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12";

/// One `visits` row as read, in [`COLUMNS`] order.
struct VisitRow {
    id: i64,
    kind: String,
    target: String,
    started_at: i64,
    last_seen_at: i64,
    title: Option<String>,
    channel: Option<(String, String, String, Option<String>)>,
    game: Option<(String, String)>,
    streamed_at: Option<i64>,
}

/// History rows are added, then only extended or removed. `touch_visit` is
/// an `UPDATE`, so it cannot bring back a visit another process removed.
impl HistoryStore for SqliteStore {
    fn load_visits(&self) -> Result<Vec<Visit>, StoreError> {
        let conn = self.conn()?;
        let rows: Vec<VisitRow> = conn
            .prepare(&format!("SELECT {COLUMNS} FROM visits {NEWEST_FIRST}"))
            .and_then(|mut stmt| stmt.query_map([], read_row)?.collect())
            .map_err(db_error)?;
        rows.into_iter().map(VisitRow::into_visit).collect()
    }

    fn latest_visit(&self) -> Result<Option<Visit>, StoreError> {
        let row = self
            .conn()?
            .query_row(
                &format!("SELECT {COLUMNS} FROM visits {NEWEST_FIRST} LIMIT 1"),
                [],
                read_row,
            )
            .optional()
            .map_err(db_error)?;
        row.map(VisitRow::into_visit).transpose()
    }

    fn add_visit(&mut self, visit: &NewVisit) -> Result<VisitId, StoreError> {
        let conn = self.conn()?;
        insert(
            &conn,
            &format!("INSERT INTO visits ({WRITTEN}) VALUES ({VALUES})"),
            visit,
        )
        .map_err(db_error)?;
        Ok(VisitId::from_raw(conn.last_insert_rowid()))
    }

    fn touch_visit(&mut self, id: VisitId, at: Timestamp) -> Result<bool, StoreError> {
        self.conn()?
            .execute(
                "UPDATE visits SET last_seen_at = MAX(last_seen_at, ?2) WHERE id = ?1",
                (id.get(), at.unix()),
            )
            .map(|changed| changed > 0)
            .map_err(db_error)
    }

    fn remove_visit(&mut self, id: VisitId) -> Result<(), StoreError> {
        self.conn()?
            .execute("DELETE FROM visits WHERE id = ?1", [id.get()])
            .map(drop)
            .map_err(db_error)
    }

    fn remove_visits_to(&mut self, target: &PlaybackTarget) -> Result<(), StoreError> {
        let (kind, target) = target_columns(target);
        self.conn()?
            .execute(
                "DELETE FROM visits WHERE kind = ?1 AND target = ?2",
                (kind, target),
            )
            .map(drop)
            .map_err(db_error)
    }

    fn clear_visits(&mut self) -> Result<(), StoreError> {
        self.conn()?
            .execute("DELETE FROM visits", [])
            .map(drop)
            .map_err(db_error)
    }
}

/// Stores `visit` unless one to the same target started at the same time
/// is stored, so importing the same visits twice adds them once. Returns
/// the rows inserted, 0 or 1.
pub(crate) fn insert_unless_stored(conn: &Connection, visit: &NewVisit) -> rusqlite::Result<usize> {
    insert(
        conn,
        &format!(
            "INSERT INTO visits ({WRITTEN}) SELECT {VALUES}
             WHERE NOT EXISTS (
                 SELECT 1 FROM visits WHERE kind = ?1 AND target = ?2 AND started_at = ?3
             )"
        ),
        visit,
    )
}

/// Runs `sql`, an insert taking [`WRITTEN`] as `?1` to `?12`, for `visit`.
fn insert(conn: &Connection, sql: &str, visit: &NewVisit) -> rusqlite::Result<usize> {
    let (kind, target) = target_columns(&visit.target);
    let details = &visit.details;
    let channel = details.channel.as_ref();
    let game = details.game.as_ref();
    conn.execute(
        sql,
        rusqlite::params![
            kind,
            target,
            visit.started_at.unix(),
            visit.last_seen_at.unix(),
            details.title,
            channel.map(|c| c.id.as_str()),
            channel.map(|c| c.login.as_str()),
            channel.map(|c| c.display_name.as_str()),
            channel.and_then(Channel::avatar),
            game.map(|g| g.id.as_str()),
            game.map(|g| g.name.as_str()),
            details.streamed_at.map(Timestamp::unix),
        ],
    )
}

fn target_columns(target: &PlaybackTarget) -> (&'static str, &str) {
    match target {
        PlaybackTarget::Live(login) => ("live", login.as_str()),
        PlaybackTarget::Vod(id) => ("vod", id.as_str()),
    }
}

fn read_row(r: &Row<'_>) -> rusqlite::Result<VisitRow> {
    let channel_id: Option<String> = r.get(6)?;
    let channel = match channel_id {
        Some(id) => Some((id, r.get(7)?, r.get(8)?, r.get(9)?)),
        None => None,
    };
    let game_id: Option<String> = r.get(10)?;
    let game = match game_id {
        Some(id) => Some((id, r.get(11)?)),
        None => None,
    };
    Ok(VisitRow {
        id: r.get(0)?,
        kind: r.get(1)?,
        target: r.get(2)?,
        started_at: r.get(3)?,
        last_seen_at: r.get(4)?,
        title: r.get(5)?,
        channel,
        game,
        streamed_at: r.get(12)?,
    })
}

impl VisitRow {
    fn into_visit(self) -> Result<Visit, StoreError> {
        let target = match self.kind.as_str() {
            "live" => PlaybackTarget::Live(
                ChannelLogin::parse(&self.target)
                    .map_err(|_| bad_row("visits", "target", &self.target))?,
            ),
            "vod" => PlaybackTarget::Vod(
                VideoId::parse(&self.target)
                    .map_err(|_| bad_row("visits", "target", &self.target))?,
            ),
            other => return Err(bad_row("visits", "kind", other)),
        };
        let channel = self
            .channel
            .map(|(id, login, display_name, avatar)| {
                Ok::<_, StoreError>(Channel {
                    id: ChannelId::parse(&id).map_err(|_| bad_row("visits", "channel_id", &id))?,
                    login: ChannelLogin::parse(&login)
                        .map_err(|_| bad_row("visits", "channel_login", &login))?,
                    display_name,
                    avatar_url: avatar.unwrap_or_default(),
                })
            })
            .transpose()?;
        let game = self
            .game
            .map(|(id, name)| {
                Ok::<_, StoreError>(GameRef {
                    id: CategoryId::parse(&id).map_err(|_| bad_row("visits", "game_id", &id))?,
                    name,
                })
            })
            .transpose()?;
        Ok(Visit {
            id: VisitId::from_raw(self.id),
            target,
            started_at: Timestamp::from_unix(self.started_at),
            last_seen_at: Timestamp::from_unix(self.last_seen_at),
            details: VisitDetails {
                channel,
                title: self.title,
                game,
                streamed_at: self.streamed_at.map(Timestamp::from_unix),
            },
        })
    }
}
