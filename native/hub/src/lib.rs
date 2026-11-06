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
// History
mod history;
// Logs
mod logs;
// Playback
mod playback;
mod probes;
mod run;
// Search
mod search;
// Settings
mod settings;
mod signals;

use std::path::PathBuf;

use jitter_core::ports::StoreError;
use jitter_store::SqliteStore;
use jitter_twitch::Twitch;
use rinf::{DartSignal, RustSignal, dart_shutdown, write_interface};

use crate::format::twitch_error;
use crate::logs::{SessionLog, Started};
use crate::run::Controllers;
use crate::signals::{Boot, BootFailed, Booted};

write_interface!();

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tokio::spawn(boot_then_run());
    // Keep the main function running until Dart shutdown.
    dart_shutdown().await;
}

/// Waits for `Boot`, starts this run's log, opens and loads the store,
/// sends `Booted`, then runs the hub task. A failed boot sends
/// `BootFailed` and waits for the next `Boot`. The hub task takes over the
/// `Boot` receiver (rinf keeps one per type) and answers a later `Boot`,
/// from a hot restart, with `Booted`. A log that did not start does not
/// stop the boot: the logs page says why.
async fn boot_then_run() {
    let boot = Boot::get_dart_signal_receiver();
    let mut log: Started = Err("no Boot yet".to_owned());
    while let Some(pack) = boot.recv().await {
        let dir = &pack.message.dir;
        if log.is_err() {
            log = start_log(dir);
        }
        match boot_at(dir) {
            Ok((controllers, twitch)) => {
                Booted.send_signal_to_dart();
                return run::run(controllers, twitch, boot, log).await;
            }
            Err(message) => {
                log::error!("boot failed: {message}");
                BootFailed { message }.send_signal_to_dart();
            }
        }
    }
}

/// Starts the log in `logs/` beside the database: on desktop in the data
/// directory every head shares, on mobile in the one Dart gave.
fn start_log(dir: &str) -> Started {
    let db = db_path(dir).map_err(|e| e.to_string())?;
    let data = db
        .parent()
        .ok_or_else(|| format!("{} has no parent directory", db.display()))?;
    SessionLog::start(data.join("logs"))
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
