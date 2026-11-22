//! The downloads the GUI lists: one entry per started download, with its progress and how it ended.
//! Finished ones are stored, so the list shows them again after a restart.

use crate::download::{Download, DownloadProgress};
use crate::models::{FinishedDownload, VideoId};
use crate::ports::{DownloadsStore, StoreError};

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

/// The list of downloads the GUI shows. Only the finished ones are stored:
/// a running one picks up from its checkpoint when it is started again, and
/// a failed or cancelled one has nothing to show after a restart.
pub struct DownloadsController {
    entries: Vec<DownloadEntry>,
    next: u64,
    store: Box<dyn DownloadsStore>,
}

impl DownloadsController {
    /// Starts empty; call [`DownloadsController::load`] to read the store.
    pub fn new(store: Box<dyn DownloadsStore>) -> Self {
        Self {
            entries: Vec::new(),
            next: 0,
            store,
        }
    }

    /// Replaces the list with the stored downloads, oldest first, each under
    /// a fresh key.
    pub fn load(&mut self) -> Result<(), StoreError> {
        let stored = self.store.load_downloads()?;
        self.entries = stored
            .into_iter()
            .map(|download| {
                let key = self.next_key();
                DownloadEntry {
                    key,
                    video: download.video,
                    file_name: download.file_name,
                    progress: DownloadProgress {
                        done: download.segments,
                        total: download.segments,
                        bytes: download.bytes,
                    },
                    status: DownloadStatus::Done {
                        path: download.path,
                    },
                }
            })
            .collect();
        Ok(())
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
        let key = self.next_key();
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

    /// Marks a Running entry Done and stores it; returns whether anything
    /// changed. It is Done even when the store fails, since its file is
    /// complete either way: the `Err` says it will not be listed after a
    /// restart.
    pub fn finish(&mut self, key: DownloadKey, path: String) -> Result<bool, StoreError> {
        let Some(entry) = self.running_mut(key) else {
            return Ok(false);
        };
        let finished = FinishedDownload {
            video: entry.video.clone(),
            file_name: entry.file_name.clone(),
            path: path.clone(),
            segments: entry.progress.total,
            bytes: entry.progress.bytes,
        };
        entry.status = DownloadStatus::Done { path };
        self.store.put_download(&finished)?;
        Ok(true)
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

    /// Drops an entry that is no longer Running (the GUI's "clear"); a
    /// Running one stays. A Done one is forgotten by the store too, and its
    /// file is kept. Returns whether it was dropped; when the store fails it
    /// stays, so the list never hides what comes back after a restart.
    pub fn remove(&mut self, key: DownloadKey) -> Result<bool, StoreError> {
        let Some(index) = self
            .entries
            .iter()
            .position(|e| e.key == key && e.status != DownloadStatus::Running)
        else {
            return Ok(false);
        };
        if let Some(DownloadEntry {
            status: DownloadStatus::Done { path },
            ..
        }) = self.entries.get(index)
        {
            self.store.remove_download(path)?;
        }
        self.entries.remove(index);
        Ok(true)
    }

    /// The file of `video`'s newest finished download, which plays in place
    /// of the stream; `None` when it has none.
    pub fn downloaded(&self, video: &VideoId) -> Option<&str> {
        self.entries.iter().rev().find_map(|e| match &e.status {
            DownloadStatus::Done { path } if e.video == *video => Some(path.as_str()),
            _ => None,
        })
    }

    pub fn entries(&self) -> &[DownloadEntry] {
        &self.entries
    }

    fn next_key(&mut self) -> DownloadKey {
        self.next = self.next.saturating_add(1);
        DownloadKey(self.next)
    }

    fn running_mut(&mut self, key: DownloadKey) -> Option<&mut DownloadEntry> {
        self.entries
            .iter_mut()
            .find(|e| e.key == key && e.status == DownloadStatus::Running)
    }
}
