//! The hub task: owns every controller, turns each Dart signal into one
//! controller action, and sends the feature's state after it. One task, so
//! actions run one at a time and need no locking. It decides nothing.
//!
//! Network calls never run inside a `select!` arm: an arm spawns the call
//! and its result comes back on the `fetched` channel as a [`Fetched`],
//! handled by its own arm. The loop stays free for the next signal while
//! a call is in flight, and core's generation and target guards make a
//! late result harmless.

use jitter_core::browse::BrowseController;
use jitter_core::favorites::FavoritesController;
use jitter_core::models::{Page, PageRequest, Playback, Stream};
use jitter_core::playback::{PlaybackController, PlaybackTarget};
use jitter_core::ports::StoreError;
use jitter_core::resume::ResumeController;
use jitter_store::SqliteStore;
use jitter_twitch::{Twitch, TwitchError};
use rinf::{DartSignal, DartSignalPack, RustSignal, SignalReceiver};
use tokio::sync::mpsc;

use crate::signals::{Boot, Booted};
// Browse
use crate::browse;
use crate::signals::{BrowseReady, NextBrowsePage, RefreshBrowse};
// Chat
use crate::chat::{self, ChatFetched, ChatPanel};
use crate::signals::{ChatReady, SetTheme};
// Favorites
use crate::favorites::{self, FavoritesFetched};
use crate::signals::{AddFavorite, FavoritesReady, RefreshFavorites, RemoveFavorite};
// Playback
use crate::playback;
use crate::signals::{LeavePlayer, Play, PlayerPosition, SelectQuality};
// Channel
use crate::channel::{self, ChannelFetched, ChannelPage};
use crate::signals::{OpenChannel, RefreshChannel, SelectVodKind, VodsNextPage};

/// Every controller, built and loaded from one store.
pub struct Controllers {
    favorites: FavoritesController,
    // Browse
    browse: BrowseController,
    // Playback
    resume: ResumeController,
    playback: PlaybackController,
    // Channel
    channel: ChannelPage,
    // Chat
    chat: ChatPanel,
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
            // Browse
            browse: BrowseController::new(),
            // Playback
            playback: PlaybackController::new(),
            // Channel
            channel: ChannelPage::default(),
            // Chat
            chat: ChatPanel::new(),
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
pub enum Fetched {
    // Browse
    TopStreams {
        request: PageRequest,
        result: Result<Page<Stream>, String>,
    },
    // Favorites
    Favorites(FavoritesFetched),
    // Playback
    Playback {
        target: PlaybackTarget,
        result: Result<Playback, TwitchError>,
    },
    // Channel
    Channel(ChannelFetched),
    // Chat
    Chat(ChatFetched),
}

/// Runs until the runtime shuts down. Holds the only receiver of every Dart
/// signal: `boot` is handed over from boot, the rest are taken here.
/// Signals sent before it starts are queued, not lost. `twitch` is cloned
/// into each spawned call; clones share one connection pool.
pub async fn run(mut c: Controllers, twitch: Twitch, boot: SignalReceiver<DartSignalPack<Boot>>) {
    // Kept alive for the loop's lifetime: arms clone it into spawned calls.
    let (fetch, mut fetched) = mpsc::unbounded_channel::<Fetched>();

    // Browse: the Top page opens first, so its first page loads at start.
    let browse_ready = BrowseReady::get_dart_signal_receiver();
    let refresh_browse = RefreshBrowse::get_dart_signal_receiver();
    let next_browse_page = NextBrowsePage::get_dart_signal_receiver();
    browse::refresh(&mut c.browse, &twitch, &fetch);

    // Favorites
    let favorites_ready = FavoritesReady::get_dart_signal_receiver();
    let add_favorite = AddFavorite::get_dart_signal_receiver();
    let remove_favorite = RemoveFavorite::get_dart_signal_receiver();
    let refresh_favorites = RefreshFavorites::get_dart_signal_receiver();

    // Playback
    let play = Play::get_dart_signal_receiver();
    let select_quality = SelectQuality::get_dart_signal_receiver();
    let player_position = PlayerPosition::get_dart_signal_receiver();
    let leave_player = LeavePlayer::get_dart_signal_receiver();
    // Channel
    let open_channel = OpenChannel::get_dart_signal_receiver();
    let refresh_channel = RefreshChannel::get_dart_signal_receiver();
    let select_vod_kind = SelectVodKind::get_dart_signal_receiver();
    let vods_next_page = VodsNextPage::get_dart_signal_receiver();

    // Chat
    let chat_ready = ChatReady::get_dart_signal_receiver();
    let set_theme = SetTheme::get_dart_signal_receiver();

    loop {
        // Chat: the batch deadline, copied out so its timer arm borrows nothing.
        let chat_flush_at = chat::flush_at(&c.chat);
        tokio::select! {
            // Boot: already booted; a hot-restarted Dart side asks again.
            Some(_) = boot.recv() => Booted.send_signal_to_dart(),

            // Browse
            Some(_) = browse_ready.recv() => browse::ready(&c.browse),
            Some(_) = refresh_browse.recv() => browse::refresh(&mut c.browse, &twitch, &fetch),
            Some(_) = next_browse_page.recv() => browse::next(&mut c.browse, &twitch, &fetch),

            // Favorites
            Some(_) = favorites_ready.recv() => favorites::on_ready(&c.favorites, &twitch, &fetch),
            Some(pack) = add_favorite.recv() => favorites::add(&pack.message.login, &twitch, &fetch),
            Some(pack) = remove_favorite.recv() => {
                favorites::remove(&mut c.favorites, &pack.message.channel_id);
            }
            Some(_) = refresh_favorites.recv() => {
                favorites::refresh(&mut c.favorites, &twitch, &fetch);
            }

            // Playback
            Some(pack) = play.recv() => {
                let target = playback::play(&mut c.playback, &twitch, &fetch, pack.message.target);
                // Chat: follows the player.
                chat::play(&mut c.chat, &twitch, &fetch, target.as_ref());
            }
            Some(pack) = select_quality.recv() => {
                playback::select_quality(&mut c.playback, pack.message);
            }
            Some(pack) = player_position.recv() => {
                playback::position(&c.playback, &mut c.resume, pack.message);
            }
            Some(pack) = leave_player.recv() => {
                // Chat: closes when the player stops.
                if playback::leave(&mut c.playback, &mut c.resume, pack.message) {
                    chat::stop(&mut c.chat);
                }
            }

            // Channel
            Some(pack) = open_channel.recv() => {
                let login = &pack.message.login;
                channel::open(&mut c.channel, &c.resume, login, &twitch, &fetch);
            }
            Some(_) = refresh_channel.recv() => {
                channel::refresh(&mut c.channel, &c.resume, &twitch, &fetch);
            }
            Some(pack) = select_vod_kind.recv() => {
                let kind = pack.message.kind;
                channel::select_kind(&mut c.channel, &c.resume, kind, &twitch, &fetch);
            }
            Some(_) = vods_next_page.recv() => {
                channel::next(&mut c.channel, &c.resume, &twitch, &fetch);
            }

            // Chat
            Some(_) = chat_ready.recv() => chat::ready(&mut c.chat),
            Some(pack) = set_theme.recv() => chat::set_theme(&mut c.chat, pack.message.dark),
            () = chat::until(chat_flush_at) => chat::flush(&mut c.chat),

            // Network results
            Some(result) = fetched.recv() => on_fetched(&mut c, result, &twitch, &fetch),

            else => break,
        }
    }
}

/// Hands a network result to the controller that asked for it, then sends
/// that feature's state. A result may start the call that follows it.
fn on_fetched(
    c: &mut Controllers,
    result: Fetched,
    twitch: &Twitch,
    fetch: &mpsc::UnboundedSender<Fetched>,
) {
    match result {
        // Browse
        Fetched::TopStreams { request, result } => browse::on_page(&mut c.browse, request, result),
        // Favorites
        Fetched::Favorites(fetched) => favorites::on_fetched(&mut c.favorites, fetched),
        // Playback
        Fetched::Playback { target, result } => {
            playback::resolved(&mut c.playback, &c.resume, target, result);
        }
        // Channel
        Fetched::Channel(fetched) => {
            channel::on_fetched(&mut c.channel, &c.resume, fetched, twitch, fetch);
        }
        // Chat
        Fetched::Chat(fetched) => chat::on_fetched(&mut c.chat, fetched),
    }
}
