//! The head loop: owns every controller, turns each [`Event`] into one
//! controller action, and shows the feature's state after it. One task on
//! the GTK main thread, so actions run one at a time and need no locking.
//! It decides nothing.
//!
//! Network calls never run in the loop: an action spawns the call on the
//! tokio runtime and its result comes back as an [`Event`] of its own.
//! Core's generation and target guards make a late result harmless.

use std::future::Future;
use std::time::Duration;

use jitter_core::browse::BrowseController;
use jitter_core::models::{ChannelLogin, Page, PageRequest, Playback, PlaybackDetails, Stream};
use jitter_core::playback::{Attempt, PlaybackController, PlaybackTarget, RESOLVE_TIMEOUT};
use jitter_core::resume::ResumeController;
use jitter_core::settings::SettingsController;
use jitter_store::SqliteStore;
use jitter_twitch::{Twitch, TwitchError};
use tokio::runtime::Handle;

use crate::view::View;

/// Every controller this head drives, built and loaded from one store.
pub struct Controllers {
    browse: BrowseController,
    playback: PlaybackController,
    resume: ResumeController,
    settings: SettingsController,
}

/// Opens the database every head on this machine shares, loads the
/// settings, and builds the Twitch client.
pub fn boot() -> Result<(Controllers, Twitch), String> {
    let path = jitter_store::default_db_path().map_err(|e| e.to_string())?;
    let store = SqliteStore::open(path).map_err(|e| format!("opening the database: {e}"))?;
    let mut settings = SettingsController::new(Box::new(store.clone()));
    settings
        .load()
        .map_err(|e| format!("loading the settings: {e}"))?;
    let twitch = Twitch::new().map_err(|e| e.to_string())?;
    let controllers = Controllers {
        browse: BrowseController::new(),
        playback: PlaybackController::new(),
        resume: ResumeController::new(Box::new(store)),
        settings,
    };
    Ok((controllers, twitch))
}

/// What the loop reacts to: an intent from a widget, or a network result
/// carrying the core types twitch returned and whatever its controller
/// needs to place it.
pub enum Event {
    // Browse
    RefreshBrowse,
    NextBrowsePage,
    TopStreams {
        request: PageRequest,
        result: Result<Page<Stream>, String>,
    },
    // Playback
    Play(ChannelLogin),
    SelectQuality(String),
    LeavePlayer(PlaybackTarget),
    Resolved {
        target: PlaybackTarget,
        result: Result<Playback, TwitchError>,
        details: Box<Result<PlaybackDetails, TwitchError>>,
    },
    ResolveTimedOut(Attempt),
}

/// Sends `event` into the loop; a closed loop (the window is gone) drops it.
pub fn send(events: &async_channel::Sender<Event>, event: Event) {
    let _ = events.try_send(event);
}

/// What an action needs to start a network call.
pub struct Context {
    pub twitch: Twitch,
    pub runtime: Handle,
    pub events: async_channel::Sender<Event>,
}

impl Context {
    /// Runs `call` on the network runtime; the event it ends in comes back
    /// into the loop.
    fn spawn(&self, call: impl Future<Output = Event> + Send + 'static) {
        let events = self.events.clone();
        self.runtime.spawn(async move {
            let _ = events.send(call.await).await;
        });
    }
}

pub async fn run(
    mut c: Controllers,
    context: Context,
    events: async_channel::Receiver<Event>,
    view: View,
) {
    while let Ok(event) = events.recv().await {
        match event {
            // Browse
            Event::RefreshBrowse => {
                let request = c.browse.begin_refresh();
                fetch_top_streams(&context, request);
                view.browse.show(c.browse.state());
            }
            Event::NextBrowsePage => {
                if let Some(request) = c.browse.begin_next() {
                    fetch_top_streams(&context, request);
                    view.browse.show(c.browse.state());
                }
            }
            Event::TopStreams { request, result } => {
                c.browse.apply_page(request, result);
                view.browse.show(c.browse.state());
            }
            // Playback
            Event::Play(login) => {
                let target = PlaybackTarget::Live(login);
                let attempt = c.playback.begin(target.clone());
                view.open_player(target.clone());
                view.player.show(c.playback.state());
                resolve(&context, target, attempt);
            }
            Event::Resolved {
                target,
                result,
                details,
            } => {
                if let Err(e) = &*details {
                    eprintln!("jitter: playback details unavailable: {e}");
                }
                let applied = c.playback.apply(
                    target,
                    result.map_err(|e| e.to_string()),
                    (*details).map_err(|e| e.to_string()),
                    c.settings.state().start_quality,
                );
                if applied {
                    match c.playback.open(&c.resume) {
                        Ok(Some(open)) => view.player.open(&open),
                        Ok(None) => {}
                        Err(e) => eprintln!("jitter: reading the resume point: {e}"),
                    }
                    view.player.show(c.playback.state());
                }
            }
            Event::ResolveTimedOut(attempt) => {
                if c.playback.time_out(attempt) {
                    view.player.show(c.playback.state());
                }
            }
            Event::SelectQuality(name) => {
                if let Some(open) = c.playback.switch_quality(&name, view.player.position()) {
                    view.player.open(&open);
                }
                view.player.show(c.playback.state());
            }
            Event::LeavePlayer(target) => {
                if c.playback.leave(&target) {
                    view.player.stop();
                }
            }
        }
    }
}

fn fetch_top_streams(context: &Context, request: PageRequest) {
    let twitch = context.twitch.clone();
    context.spawn(async move {
        let result = twitch
            .top_streams(request.cursor.as_ref())
            .await
            .map_err(|e| e.to_string());
        Event::TopStreams { request, result }
    });
}

/// Resolves `target` and loads what it plays beside it; both come back
/// together as [`Event::Resolved`], or as [`Event::ResolveTimedOut`] when
/// they take longer than core's `RESOLVE_TIMEOUT`.
fn resolve(context: &Context, target: PlaybackTarget, attempt: Attempt) {
    let twitch = context.twitch.clone();
    context.spawn(async move {
        let resolve = async {
            match &target {
                PlaybackTarget::Live(login) => twitch.live(login).await,
                PlaybackTarget::Vod(id) => twitch.vod(id).await,
            }
        };
        let both = async { tokio::join!(resolve, twitch.playback_details(&target)) };
        let limit = Duration::from_secs(u64::from(RESOLVE_TIMEOUT.get()));
        match tokio::time::timeout(limit, both).await {
            Ok((result, details)) => Event::Resolved {
                target,
                result,
                details: Box::new(details),
            },
            Err(_) => Event::ResolveTimedOut(attempt),
        }
    });
}
