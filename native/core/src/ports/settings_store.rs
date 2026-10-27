use crate::models::{Setting, Settings};
use crate::ports::StoreError;

/// The user's persistent settings. Owned by `SettingsController`, which
/// writes through it before changing its state.
///
/// The database is shared by every head on the machine, so a write changes
/// one setting and leaves the others as stored: a head never overwrites a
/// setting another head changed since it loaded.
pub trait SettingsStore: Send {
    /// The stored settings; any never stored has its
    /// [`Settings::default`] value.
    fn load_settings(&self) -> Result<Settings, StoreError>;

    /// Stores `setting`, replacing its stored value.
    fn put_setting(&mut self, setting: Setting) -> Result<(), StoreError>;
}
