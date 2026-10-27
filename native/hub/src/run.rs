//! The hub task: owns every controller, turns each Dart signal into one
//! controller action, and sends the feature's state after it. One task, so
//! actions run one at a time and need no locking. It decides nothing.
//!
//! Network calls never run inside a `select!` arm: an arm spawns the call
//! and its result comes back on the `fetched` channel as a [`Fetched`],
//! handled by its own arm. The loop stays free for the next signal while
//! a call is in flight, and core's generation and target guards make a
//! late result harmless.

use jitter_core::favorites::FavoritesController;
use jitter_core::ports::StoreError;
use jitter_core::resume::ResumeController;
use jitter_store::SqliteStore;
use rinf::{DartSignalPack, RustSignal, SignalReceiver};
use tokio::sync::mpsc;

use crate::signals::{Boot, Booted};

/// Every controller, built and loaded from one store.
#[expect(
    dead_code,
    reason = "read by the Favorites (G4) and player (G7) arms when they land"
)]
pub struct Controllers {
    favorites: FavoritesController,
    resume: ResumeController,
}

impl Controllers {
    /// Builds each controller on a clone of `store`, then loads the
    /// favorites.
    pub fn boot(store: &SqliteStore) -> Result<Self, StoreError> {
        let mut favorites = FavoritesController::new(Box::new(store.clone()));
        favorites.load()?;
        Ok(Self {
            favorites,
            resume: ResumeController::new(Box::new(store.clone())),
        })
    }
}

/// A network result on its way back into the loop: one variant per call,
/// carrying the core types twitch returned (or its error) and whatever the
/// owning controller needs to place it (a generation, a target id).
///
/// To add a fetch: add a variant here, and in the arm that starts the call
/// spawn it with a clone of `fetch`:
///
/// ```ignore
/// let (twitch, fetch) = (twitch.clone(), fetch.clone());
/// tokio::spawn(async move {
///     let result = twitch.top_streams(cursor).await;
///     let _ = fetch.send(Fetched::TopStreams { generation, result });
/// });
/// ```
///
/// then hand the variant to its controller in the `fetched` arm's `match`.
/// Uninhabited until the first feature fetches.
pub enum Fetched {}

/// Runs until the runtime shuts down. Holds the only receiver of every Dart
/// signal: `boot` is handed over from boot, the rest are taken here.
/// Signals sent before it starts are queued, not lost.
pub async fn run(mut c: Controllers, boot: SignalReceiver<DartSignalPack<Boot>>) {
    // Kept alive for the loop's lifetime: arms clone it into spawned calls.
    let (_fetch, mut fetched) = mpsc::unbounded_channel::<Fetched>();

    loop {
        tokio::select! {
            // Boot: already booted; a hot-restarted Dart side asks again.
            Some(_) = boot.recv() => Booted.send_signal_to_dart(),

            // Network results
            Some(result) = fetched.recv() => on_fetched(&mut c, result),

            else => break,
        }
    }
}

/// Hands a network result to the controller that asked for it, then sends
/// that feature's state.
fn on_fetched(_c: &mut Controllers, result: Fetched) {
    match result {}
}
