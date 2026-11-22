use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::models::FinishedDownload;
use crate::ports::{DownloadsStore, StoreError};

/// In-memory [`DownloadsStore`]. Can be told to fail loads or writes.
#[derive(Debug, Clone, Default)]
pub struct FakeDownloadsStore {
    downloads: Arc<Mutex<Vec<FinishedDownload>>>,
    fail_loads: bool,
    fail_writes: bool,
}

impl FakeDownloadsStore {
    /// A store already holding `downloads`, oldest first.
    pub fn with_downloads(downloads: Vec<FinishedDownload>) -> Self {
        Self {
            downloads: Arc::new(Mutex::new(downloads)),
            ..Self::default()
        }
    }

    /// Every `load_downloads` returns `Err`.
    pub fn failing_loads(self) -> Self {
        Self {
            fail_loads: true,
            ..self
        }
    }

    /// Every `put_download` and `remove_download` returns `Err` and stores
    /// nothing.
    pub fn failing_writes(self) -> Self {
        Self {
            fail_writes: true,
            ..self
        }
    }

    /// What is stored now, oldest first.
    pub fn downloads(&self) -> Vec<FinishedDownload> {
        self.lock().clone()
    }

    fn lock(&self) -> MutexGuard<'_, Vec<FinishedDownload>> {
        self.downloads
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn check_write(&self) -> Result<(), StoreError> {
        if self.fail_writes {
            return Err(StoreError::new("fake write failure"));
        }
        Ok(())
    }
}

impl DownloadsStore for FakeDownloadsStore {
    fn load_downloads(&self) -> Result<Vec<FinishedDownload>, StoreError> {
        if self.fail_loads {
            return Err(StoreError::new("fake load failure"));
        }
        Ok(self.downloads())
    }

    fn put_download(&mut self, download: &FinishedDownload) -> Result<(), StoreError> {
        self.check_write()?;
        let mut downloads = self.lock();
        match downloads.iter_mut().find(|d| d.path == download.path) {
            Some(stored) => *stored = download.clone(),
            None => downloads.push(download.clone()),
        }
        Ok(())
    }

    fn remove_download(&mut self, path: &str) -> Result<(), StoreError> {
        self.check_write()?;
        self.lock().retain(|d| d.path != path);
        Ok(())
    }
}
