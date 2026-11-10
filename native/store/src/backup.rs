use jitter_core::models::Timestamp;
use jitter_core::ports::{FavoritesStore, HistoryStore, SettingsStore, StoreError};
use rusqlite::TransactionBehavior;

use crate::imported::Imported;
use crate::sqlite_store::{SqliteStore, db_error};
use crate::{backup_file, favorite_rows, history_rows, resume_rows, settings_rows};

/// Backups ([`backup_file`]). An import merges into what is stored, so it
/// never removes anything, and importing the same backup again adds
/// nothing: a stored favorite or visit is kept as it is, a resume point is
/// replaced only by a newer one, and the backup's settings are taken.
impl SqliteStore {
    /// Everything stored, as a backup file's text, stamped `now`.
    pub fn export_backup(&self, now: Timestamp) -> Result<String, StoreError> {
        let favorites = self.load_all()?;
        let visits = self.load_visits()?;
        let settings = self.load_settings()?;
        let resume_points = resume_rows::all(&*self.conn()?)?;
        backup_file::write(now, &favorites, &visits, &resume_points, &settings)
    }

    /// Merges the backup `json` into the database in one transaction: an
    /// invalid backup, or a failed write, changes nothing.
    pub fn import_backup(&mut self, json: &str) -> Result<Imported, StoreError> {
        let contents = backup_file::read(json)?;
        let mut conn = self.conn()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        let mut imported = Imported::default();
        for favorite in &contents.favorites {
            imported.favorites += favorite_rows::insert(&tx, favorite).map_err(db_error)?;
        }
        for visit in &contents.visits {
            imported.visits += history_rows::insert_unless_stored(&tx, visit).map_err(db_error)?;
        }
        for point in &contents.resume_points {
            imported.resume_points += resume_rows::put_if_newer(&tx, point).map_err(db_error)?;
        }
        for setting in contents.settings {
            settings_rows::put(&tx, setting).map_err(db_error)?;
        }
        tx.commit().map_err(db_error)?;
        Ok(imported)
    }
}
