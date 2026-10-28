//! Storage ports: the border between core and the disk, which cannot run in
//! a test. Controllers take them as `Box<dyn Trait>` in their constructor;
//! `jitter_store` implements them over SQLite; the `fakes` module holds the
//! test doubles.
//!
//! Each trait requires `Send`, so the controllers holding them can move into
//! a head's spawned task.

mod favorites_store;
mod history_store;
mod resume_store;
mod settings_store;
mod store_error;

pub use favorites_store::FavoritesStore;
pub use history_store::HistoryStore;
pub use resume_store::ResumeStore;
pub use settings_store::SettingsStore;
pub use store_error::StoreError;
