use jitter_core::models::{ChannelId, ChannelLogin, Favorite, Timestamp};
use jitter_core::ports::{FavoritesStore, StoreError};

use crate::sqlite_store::{SqliteStore, bad_row, db_error};

/// The database is shared with other processes, so no statement here is an
/// upsert: `add` inserts only if absent, `rename` only updates. Neither can
/// bring back a favorite another process removed.
impl FavoritesStore for SqliteStore {
    fn load_all(&self) -> Result<Vec<Favorite>, StoreError> {
        let conn = self.conn()?;
        let rows: Vec<(String, String, String, i64)> = conn
            .prepare(
                "SELECT channel_id, login, display_name, added_at
                 FROM favorites ORDER BY added_at, channel_id",
            )
            .and_then(|mut stmt| {
                stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
                    .collect()
            })
            .map_err(db_error)?;
        rows.into_iter()
            .map(|(id, login, display_name, added_at)| {
                Ok(Favorite {
                    channel_id: ChannelId::parse(&id)
                        .map_err(|_| bad_row("favorites", "channel_id", &id))?,
                    login: ChannelLogin::parse(&login)
                        .map_err(|_| bad_row("favorites", "login", &login))?,
                    display_name,
                    added_at: Timestamp::from_unix(added_at),
                })
            })
            .collect()
    }

    fn add(&mut self, favorite: &Favorite) -> Result<(), StoreError> {
        self.conn()?
            .execute(
                "INSERT INTO favorites (channel_id, login, display_name, added_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (channel_id) DO NOTHING",
                (
                    favorite.channel_id.as_str(),
                    favorite.login.as_str(),
                    favorite.display_name.as_str(),
                    favorite.added_at.unix(),
                ),
            )
            .map(drop)
            .map_err(db_error)
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

    fn remove(&mut self, id: &ChannelId) -> Result<(), StoreError> {
        self.conn()?
            .execute("DELETE FROM favorites WHERE channel_id = ?1", [id.as_str()])
            .map(drop)
            .map_err(db_error)
    }
}
