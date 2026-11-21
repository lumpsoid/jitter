//! The downloads the GUI lists: one entry per started download, with its progress and how it ended.

use crate::download::{Download, DownloadProgress};
use crate::models::VideoId;

/// Names one entry. Downloads run in parallel, so the GUI cancels or removes
/// one by this key rather than by its file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DownloadKey(u64);

impl DownloadKey {
    /// The hub maps keys to and from signals with these two.
    pub fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadStatus {
    Running,
    Done { path: String },
    Failed { message: String },
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadEntry {
    pub key: DownloadKey,
    pub video: VideoId,
    pub file_name: String,
    pub progress: DownloadProgress,
    pub status: DownloadStatus,
}

/// The list of downloads the GUI shows.
#[derive(Debug, Default)]
pub struct DownloadsController {
    entries: Vec<DownloadEntry>,
    next: u64,
}

impl DownloadsController {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a Running entry for `download`, newest last. None when that file
    /// is already being written.
    pub fn start(&mut self, video: VideoId, download: &Download) -> Option<DownloadKey> {
        let file_name = download.file_name();
        let busy = self
            .entries
            .iter()
            .any(|e| e.status == DownloadStatus::Running && e.file_name == file_name);
        if busy {
            return None;
        }
        self.next = self.next.saturating_add(1);
        let key = DownloadKey(self.next);
        self.entries.push(DownloadEntry {
            key,
            video,
            file_name: file_name.to_owned(),
            progress: download.progress(),
            status: DownloadStatus::Running,
        });
        Some(key)
    }

    /// Progress of a Running entry; returns whether anything changed.
    pub fn progress(&mut self, key: DownloadKey, progress: DownloadProgress) -> bool {
        let Some(entry) = self.running_mut(key) else {
            return false;
        };
        let changed = entry.progress != progress;
        entry.progress = progress;
        changed
    }

    pub fn finish(&mut self, key: DownloadKey, path: String) -> bool {
        let Some(entry) = self.running_mut(key) else {
            return false;
        };
        entry.status = DownloadStatus::Done { path };
        true
    }

    pub fn fail(&mut self, key: DownloadKey, message: String) -> bool {
        let Some(entry) = self.running_mut(key) else {
            return false;
        };
        entry.status = DownloadStatus::Failed { message };
        true
    }

    /// Marks a Running entry Cancelled; the hub stops its task when this returns true.
    pub fn cancel(&mut self, key: DownloadKey) -> bool {
        let Some(entry) = self.running_mut(key) else {
            return false;
        };
        entry.status = DownloadStatus::Cancelled;
        true
    }

    /// Drops an entry that is no longer Running (the GUI's "clear"); a Running one stays.
    pub fn remove(&mut self, key: DownloadKey) -> bool {
        let before = self.entries.len();
        self.entries
            .retain(|e| e.key != key || e.status == DownloadStatus::Running);
        self.entries.len() != before
    }

    pub fn entries(&self) -> &[DownloadEntry] {
        &self.entries
    }

    fn running_mut(&mut self, key: DownloadKey) -> Option<&mut DownloadEntry> {
        self.entries
            .iter_mut()
            .find(|e| e.key == key && e.status == DownloadStatus::Running)
    }
}
