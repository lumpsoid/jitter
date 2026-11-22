use crate::models::FinishedDownload;
use crate::ports::StoreError;

/// The finished downloads, kept so the list shows them after a restart.
/// Owned by `DownloadsController`. A download is named by its file's path.
pub trait DownloadsStore: Send {
    /// Every stored download, oldest first.
    fn load_downloads(&self) -> Result<Vec<FinishedDownload>, StoreError>;

    /// Stores `download`, replacing one at the same path.
    fn put_download(&mut self, download: &FinishedDownload) -> Result<(), StoreError>;

    /// Deletes the download at `path`. An unknown path is not an error.
    fn remove_download(&mut self, path: &str) -> Result<(), StoreError>;
}
