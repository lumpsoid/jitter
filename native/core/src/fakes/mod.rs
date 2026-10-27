//! Test doubles for the [`crate::ports`] traits. Compiled only for tests and
//! with the `test-fakes` feature, so they never reach the production API.
//! Integration tests in `native/core/tests/` get the feature through the
//! crate's self dev-dependency; another crate's tests can enable it the same
//! way.
//!
//! Each fake is a handle to shared contents: clone it before boxing it into
//! a controller and the clone shows what the controller wrote.

mod fake_favorites_store;
mod fake_resume_store;
mod fake_settings_store;

pub use fake_favorites_store::FakeFavoritesStore;
pub use fake_resume_store::FakeResumeStore;
pub use fake_settings_store::FakeSettingsStore;
