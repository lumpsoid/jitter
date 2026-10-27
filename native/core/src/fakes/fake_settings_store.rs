use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::models::{Setting, Settings};
use crate::ports::{SettingsStore, StoreError};

/// In-memory [`SettingsStore`], starting at the defaults. Can be told to
/// fail loads or writes.
#[derive(Debug, Clone, Default)]
pub struct FakeSettingsStore {
    settings: Arc<Mutex<Settings>>,
    fail_loads: bool,
    fail_writes: bool,
}

impl FakeSettingsStore {
    /// A store already holding `settings`.
    pub fn with_settings(settings: Settings) -> Self {
        Self {
            settings: Arc::new(Mutex::new(settings)),
            ..Self::default()
        }
    }

    /// Every `load_settings` returns `Err`.
    pub fn failing_loads(self) -> Self {
        Self {
            fail_loads: true,
            ..self
        }
    }

    /// Every `put_setting` returns `Err` and stores nothing.
    pub fn failing_writes(self) -> Self {
        Self {
            fail_writes: true,
            ..self
        }
    }

    /// What is stored now.
    pub fn settings(&self) -> Settings {
        *self.lock()
    }

    fn lock(&self) -> MutexGuard<'_, Settings> {
        self.settings.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl SettingsStore for FakeSettingsStore {
    fn load_settings(&self) -> Result<Settings, StoreError> {
        if self.fail_loads {
            return Err(StoreError::new("fake load failure"));
        }
        Ok(self.settings())
    }

    fn put_setting(&mut self, setting: Setting) -> Result<(), StoreError> {
        if self.fail_writes {
            return Err(StoreError::new("fake write failure"));
        }
        let mut stored = self.lock();
        *stored = stored.with(setting);
        Ok(())
    }
}
