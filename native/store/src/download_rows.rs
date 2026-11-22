use jitter_core::models::{FinishedDownload, VideoId};
use jitter_core::ports::{DownloadsStore, StoreError};

use crate::sqlite_store::{SqliteStore, bad_row, db_error};

/// Rows come back in `rowid` order, the order they were first stored: a
/// download stored again at the same path keeps its place.
impl DownloadsStore for SqliteStore {
    fn load_downloads(&self) -> Result<Vec<FinishedDownload>, StoreError> {
        let rows: Vec<(String, String, String, i64, i64)> = self
            .conn()?
            .prepare(
                "SELECT video_id, file_name, path, segments, bytes
                 FROM downloads ORDER BY rowid",
            )
            .and_then(|mut stmt| {
                stmt.query_map([], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
                })?
                .collect()
            })
            .map_err(db_error)?;
        rows.into_iter()
            .map(|(video_id, file_name, path, segments, bytes)| {
                Ok(FinishedDownload {
                    video: VideoId::parse(&video_id)
                        .map_err(|_| bad_row("downloads", "video_id", &video_id))?,
                    file_name,
                    path,
                    segments: usize::try_from(segments)
                        .map_err(|_| bad_row("downloads", "segments", segments))?,
                    bytes: u64::try_from(bytes)
                        .map_err(|_| bad_row("downloads", "bytes", bytes))?,
                })
            })
            .collect()
    }

    fn put_download(&mut self, download: &FinishedDownload) -> Result<(), StoreError> {
        let segments = i64::try_from(download.segments)
            .map_err(|_| StoreError::new("a download's segment count is past i64"))?;
        let bytes = i64::try_from(download.bytes)
            .map_err(|_| StoreError::new("a download's size is past i64"))?;
        self.conn()?
            .execute(
                "INSERT INTO downloads (path, video_id, file_name, segments, bytes)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (path) DO UPDATE SET
                     video_id  = excluded.video_id,
                     file_name = excluded.file_name,
                     segments  = excluded.segments,
                     bytes     = excluded.bytes",
                (
                    download.path.as_str(),
                    download.video.as_str(),
                    download.file_name.as_str(),
                    segments,
                    bytes,
                ),
            )
            .map(drop)
            .map_err(db_error)
    }

    fn remove_download(&mut self, path: &str) -> Result<(), StoreError> {
        self.conn()?
            .execute("DELETE FROM downloads WHERE path = ?1", [path])
            .map(drop)
            .map_err(db_error)
    }
}
