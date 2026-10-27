//! The user's settings: loaded at boot, changed one at a time, written
//! through before the state changes. Other controllers do not read this
//! one; heads pass them the settings they need as plain values.

use crate::models::{Setting, Settings};
use crate::ports::{SettingsStore, StoreError};

pub struct SettingsController {
    state: Settings,
    store: Box<dyn SettingsStore>,
}

impl SettingsController {
    /// Starts at the defaults; call [`Self::load`] to read the store.
    pub fn new(store: Box<dyn SettingsStore>) -> Self {
        Self {
            state: Settings::default(),
            store,
        }
    }

    /// Replaces the settings with the stored ones. A failed read leaves
    /// them as they were.
    pub fn load(&mut self) -> Result<(), StoreError> {
        self.state = self.store.load_settings()?;
        Ok(())
    }

    pub fn state(&self) -> &Settings {
        &self.state
    }

    /// Changes one setting: stored first, so a failed write leaves the
    /// settings unchanged and returns `Err`.
    pub fn set(&mut self, setting: Setting) -> Result<(), StoreError> {
        self.store.put_setting(setting)?;
        self.state = self.state.with(setting);
        Ok(())
    }
}
