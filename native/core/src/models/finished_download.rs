use super::VideoId;

/// A download whose file is complete, as the store keeps it so the list
/// shows it again after a restart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishedDownload {
    pub video: VideoId,
    /// `<login>_<id>_<quality>.<ext>`, as the list shows it.
    pub file_name: String,
    /// The file's full path; names the download in the store.
    pub path: String,
    /// How many segments it joined.
    pub segments: usize,
    pub bytes: u64,
}
