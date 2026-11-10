use jitter_core::models::{ChannelId, ChannelLogin, Favorite, Timestamp};
use jitter_core::ports::{FavoritesStore, StoreError};
use rusqlite::Connection;

use crate::sqlite_store::{SqliteStore, bad_row, db_error};

/// One `favorites` row as read: id, login, display name, avatar URL, added at.
type Row = (String, String, String, Option<String>, i64);

/// The database is shared with other processes, so no statement here is an
/// upsert: `add` inserts only if absent, `rename` and `set_avatar` only
/// update. None can bring back a favorite another process removed.
impl FavoritesStore for SqliteStore {
    fn load_all(&self) -> Result<Vec<Favorite>, StoreError> {
        let conn = self.conn()?;
        let rows: Vec<Row> = conn
            .prepare(
                "SELECT channel_id, login, display_name, avatar_url, added_at
                 FROM favorites ORDER BY added_at, channel_id",
            )
            .and_then(|mut stmt| {
                stmt.query_map([], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
                })?
                .collect()
            })
            .map_err(db_error)?;
        rows.into_iter()
            .map(|(id, login, display_name, avatar_url, added_at)| {
                Ok(Favorite {
                    channel_id: ChannelId::parse(&id)
                        .map_err(|_| bad_row("favorites", "channel_id", &id))?,
                    login: ChannelLogin::parse(&login)
                        .map_err(|_| bad_row("favorites", "login", &login))?,
                    display_name,
                    avatar_url,
                    added_at: Timestamp::from_unix(added_at),
                })
            })
            .collect()
    }

    fn add(&mut self, favorite: &Favorite) -> Result<(), StoreError> {
        insert(&*self.conn()?, favorite).map(drop).map_err(db_error)
    }

    fn rename(
        &mut self,
        id: &ChannelId,
        login: &ChannelLogin,
        display_name: &str,
    ) -> Result<(), StoreError> {
        self.conn()?
            .execute(
                "UPDATE favorites SET login = ?2, display_name = ?3 WHERE channel_id = ?1",
                (id.as_str(), login.as_str(), display_name),
            )
            .map(drop)
            .map_err(db_error)
    }

    fn set_avatar(&mut self, id: &ChannelId, avatar_url: &str) -> Result<(), StoreError> {
        self.conn()?
            .execute(
                "UPDATE favorites SET avatar_url = ?2 WHERE channel_id = ?1",
                (id.as_str(), avatar_url),
            )
            .map(drop)
            .map_err(db_error)
    }

    fn remove(&mut self, id: &ChannelId) -> Result<(), StoreError> {
        self.conn()?
            .execute("DELETE FROM favorites WHERE channel_id = ?1", [id.as_str()])
            .map(drop)
            .map_err(db_error)
    }
}

/// Inserts `favorite` unless its channel id is stored; a stored one is left
/// as it is. Returns the rows inserted, 0 or 1.
pub(crate) fn insert(conn: &Connection, favorite: &Favorite) -> rusqlite::Result<usize> {
    conn.execute(
        "INSERT INTO favorites (channel_id, login, display_name, avatar_url, added_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (channel_id) DO NOTHING",
        (
            favorite.channel_id.as_str(),
            favorite.login.as_str(),
            favorite.display_name.as_str(),
            favorite.avatar_url.as_deref(),
            favorite.added_at.unix(),
        ),
    )
}
