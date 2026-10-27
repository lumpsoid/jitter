//! The rinf bridge: turns Dart signals into controller actions and
//! controller state into Rust signals. Entry point of the Rust logic.

// Browse
mod browse;
// Categories
mod categories;
mod category;
// Channel
mod channel;
// Chat
mod chat;
mod favorites;
mod format;
// Playback
mod playback;
mod run;
mod signals;

use std::path::PathBuf;

use jitter_core::ports::StoreError;
use jitter_store::SqliteStore;
use jitter_twitch::Twitch;
use rinf::{DartSignal, RustSignal, dart_shutdown, write_interface};

use crate::format::twitch_error;
use crate::run::Controllers;
use crate::signals::{Boot, BootFailed, Booted};

write_interface!();

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tokio::spawn(boot_then_run());
    // Keep the main function running until Dart shutdown.
    dart_shutdown().await;
}

/// Waits for `Boot`, opens and loads the store, sends `Booted`, then runs
/// the hub task. A failed boot sends `BootFailed` and waits for the next
/// `Boot`. The hub task takes over the `Boot` receiver (rinf keeps one per
/// type) and answers a later `Boot`, from a hot restart, with `Booted`.
async fn boot_then_run() {
    let boot = Boot::get_dart_signal_receiver();
    while let Some(pack) = boot.recv().await {
        match boot_at(&pack.message.dir) {
            Ok((controllers, twitch)) => {
                Booted.send_signal_to_dart();
                return run::run(controllers, twitch, boot).await;
            }
            Err(message) => BootFailed { message }.send_signal_to_dart(),
        }
    }
}

/// Opens the store and builds the Twitch client; either failing is shown
/// to the user as the boot failure's message.
fn boot_at(dir: &str) -> Result<(Controllers, Twitch), String> {
    let open = || Controllers::boot(&SqliteStore::open(db_path(dir)?)?);
    let controllers = open().map_err(|e: StoreError| e.to_string())?;
    let twitch = Twitch::new().map_err(|e| twitch_error(&e))?;
    Ok((controllers, twitch))
}

/// Desktop: the database every head on this machine shares, so the GUI and
/// the CLI see the same favorites; `dir` is ignored.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn db_path(_dir: &str) -> Result<PathBuf, StoreError> {
    jitter_store::default_db_path()
}

/// Mobile: no location is shared with other heads, so the database lives
/// in the directory the platform gave Dart.
#[cfg(any(target_os = "android", target_os = "ios"))]
fn db_path(dir: &str) -> Result<PathBuf, StoreError> {
    Ok(std::path::Path::new(dir).join("jitter.db"))
}
