use rinf::{DartSignal, RustSignal};
use serde::{Deserialize, Serialize};

/// Sent by Dart at app start, to retry a failed boot, and again after a
/// hot restart. `dir` is the app's documents directory, resolved by Dart
/// (`path_provider`): mobile keeps the database there; desktop ignores it
/// and opens the file the CLI uses.
#[derive(Deserialize, DartSignal)]
pub struct Boot {
    pub dir: String,
}

/// The store is open and the hub task runs. Answers every [`Boot`] from
/// then on, so a hot-restarted Dart side gets it too.
#[derive(Serialize, RustSignal)]
pub struct Booted;

/// The store could not be opened or loaded. Dart shows `message` instead of
/// the spinner; it may send [`Boot`] again to retry.
#[derive(Serialize, RustSignal)]
pub struct BootFailed {
    pub message: String,
}
