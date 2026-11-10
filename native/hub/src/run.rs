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
use jitter_core::categories::TopCategoriesController;
use jitter_core::favorites::FavoritesController;
use jitter_core::history::HistoryController;
use jitter_core::models::{
    Category, Page, PageRequest, Playback, PlaybackDetails, RewindFailure, Stream, VideoId,
};
use jitter_core::playback::{Attempt, PlaybackController, PlaybackTarget};
use jitter_core::ports::StoreError;
use jitter_core::resume::ResumeController;
use jitter_core::settings::SettingsController;
use jitter_store::SqliteStore;
use jitter_twitch::{Twitch, TwitchError};
use rinf::{DartSignal, DartSignalPack, RustSignal, SignalReceiver};
use tokio::sync::mpsc;
use tokio::time::Instant;

use crate::signals::{Boot, Booted};
// Backup
use crate::backup;
use crate::signals::{ExportBackup, ImportBackup};
// Browse
use crate::browse;
use crate::signals::{BrowseReady, NextBrowsePage, RefreshBrowse};
// Categories
use crate::categories;
use crate::category::{self, CategoryPage};
use crate::signals::{
    CategoriesReady, NextCategoriesPage, NextCategoryPage, OpenCategory, RefreshCategories,
    RefreshCategory,
};
// Chat
use crate::chat::{self, ChatFetched, ChatPanel};
use crate::signals::{ChatReady, HideChat, SetTheme, ShowChat};
// Favorites
use crate::favorites::{self, FavoritesFetched};
use crate::signals::{AddFavorite, FavoritesReady, RefreshFavorites, RemoveFavorite};
// History
use crate::history;
use crate::signals::{ClearHistory, HistoryReady, RemoveFromHistory, RemoveVisit};
// Logs
use crate::logs::{self, Buffering, Started};
use crate::probes;
use crate::signals::{
    DisplaysProbe, LogFromDart, LogsReady, MpvVersions, OpenLog, PlaybackDrops, PlaybackProbe,
    PlayerBuffering,
};
// Playback
use crate::playback;
use crate::signals::{
    AppBackgrounded, AppForegrounded, ArmSleepTimer, BackToLive, GoToChapter, GoToTime,
    LeavePlayer, Play, PlayerPosition, PlayerStartedAway, RefreshPlayback, RewindStream,
    SelectQuality, TurnOffSleepTimer,
};
// Channel
use crate::channel::{self, ChannelFetched, ChannelPage};
use crate::signals::{OpenChannel, RefreshChannel, SelectVodKind, VodsNextPage};
// Search
use crate::search::{self, SearchFetched, SearchPanel};
use crate::signals::{NextSearchPage, RefreshSearch, SearchTyped};
// Settings
use crate::settings;
use crate::signals::{ChangeSetting, SettingsReady};

/// Every controller, built and loaded from one store.
pub struct Controllers {
    favorites: FavoritesController,
    // History
    history: HistoryController,
    // Browse
    browse: BrowseController,
    // Categories
    top_categories: TopCategoriesController,
    category: CategoryPage,
    // Playback
    resume: ResumeController,
    playback: PlaybackController,
    // Channel
    channel: ChannelPage,
    // Chat
    chat: ChatPanel,
    // Search
    search: SearchPanel,
    // Settings
    settings: SettingsController,
    // Backup: the store itself, which exports and imports whole.
    store: SqliteStore,
}

impl Controllers {
    /// Builds each controller on a clone of `store`, then loads the
    /// favorites, the history and the settings; the chat starts under the
    /// settings. Keeps a clone of `store` for backups.
    pub fn boot(store: &SqliteStore) -> Result<Self, StoreError> {
        let mut favorites = FavoritesController::new(Box::new(store.clone()));
        favorites.load()?;
        let mut history = HistoryController::new(Box::new(store.clone()));
        history.load()?;
        let mut settings = SettingsController::new(Box::new(store.clone()));
        settings.load()?;
        Ok(Self {
            favorites,
            // History
            history,
            resume: ResumeController::new(Box::new(store.clone())),
            // Browse
            browse: BrowseController::new(),
            // Categories
            top_categories: TopCategoriesController::new(),
            category: None,
            // Playback
            playback: PlaybackController::new(),
            // Channel
            channel: ChannelPage::default(),
            // Chat
            chat: ChatPanel::new(*settings.state()),
            // Search
            search: SearchPanel::default(),
            // Settings
            settings,
            // Backup
            store: store.clone(),
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
    // Categories
    TopCategories {
        request: PageRequest,
        result: Result<Page<Category>, String>,
    },
    CategoryStreams {
        request: PageRequest,
        result: Result<Page<Stream>, String>,
    },
    // Favorites
    Favorites(FavoritesFetched),
    // Playback
    Playback {
        target: PlaybackTarget,
        result: Result<Playback, TwitchError>,
        details: Result<PlaybackDetails, TwitchError>,
    },
    PlaybackTimedOut(Attempt),
    /// Page `target`'s details loaded afresh for `stream`, what it played
    /// when asked; a timeout arrives as a failure.
    PlaybackDetails {
        target: PlaybackTarget,
        stream: PlaybackTarget,
        details: Result<PlaybackDetails, String>,
    },
    /// Live page `target`'s recording `video` resolved for a rewind; a
    /// timeout arrives as a failure.
    Rewound {
        target: PlaybackTarget,
        video: VideoId,
        result: Result<Playback, RewindFailure>,
        details: Result<PlaybackDetails, String>,
    },
    /// Live page `target`, returning from its recording `video`, resolved
    /// live; a timeout arrives as a failure.
    Returned {
        target: PlaybackTarget,
        video: VideoId,
        result: Result<Playback, String>,
        details: Result<PlaybackDetails, String>,
    },
    /// Stalled `stream`, what a page plays, resolved afresh; a timeout
    /// arrives as a failure.
    Reloaded {
        stream: PlaybackTarget,
        result: Result<Playback, String>,
    },
    // Channel
    Channel(ChannelFetched),
    // Chat
    Chat(ChatFetched),
    // Search
    Search(SearchFetched),
}

/// Runs until the runtime shuts down. Holds the only receiver of every Dart
/// signal: `boot` is handed over from boot, the rest are taken here.
/// `log` is this run's log, or why it did not start.
/// Signals sent before it starts are queued, not lost. `twitch` is cloned
/// into each spawned call; clones share one connection pool.
pub async fn run(
    mut c: Controllers,
    twitch: Twitch,
    boot: SignalReceiver<DartSignalPack<Boot>>,
    log: Started,
) {
    // Kept alive for the loop's lifetime: arms clone it into spawned calls.
    let (fetch, mut fetched) = mpsc::unbounded_channel::<Fetched>();

    // Browse: the Top page opens first, so its first page loads at start.
    let browse_ready = BrowseReady::get_dart_signal_receiver();
    let refresh_browse = RefreshBrowse::get_dart_signal_receiver();
    let next_browse_page = NextBrowsePage::get_dart_signal_receiver();
    browse::refresh(&mut c.browse, &twitch, &fetch);

    // Categories: like Top, the first page loads at start.
    let categories_ready = CategoriesReady::get_dart_signal_receiver();
    let refresh_categories = RefreshCategories::get_dart_signal_receiver();
    let next_categories_page = NextCategoriesPage::get_dart_signal_receiver();
    let open_category = OpenCategory::get_dart_signal_receiver();
    let refresh_category = RefreshCategory::get_dart_signal_receiver();
    let next_category_page = NextCategoryPage::get_dart_signal_receiver();
    categories::refresh(&mut c.top_categories, &twitch, &fetch);

    // Favorites
    let favorites_ready = FavoritesReady::get_dart_signal_receiver();
    let add_favorite = AddFavorite::get_dart_signal_receiver();
    let remove_favorite = RemoveFavorite::get_dart_signal_receiver();
    let refresh_favorites = RefreshFavorites::get_dart_signal_receiver();

    // History
    let history_ready = HistoryReady::get_dart_signal_receiver();
    let remove_visit = RemoveVisit::get_dart_signal_receiver();
    let remove_from_history = RemoveFromHistory::get_dart_signal_receiver();
    let clear_history = ClearHistory::get_dart_signal_receiver();

    // Logs
    let logs_ready = LogsReady::get_dart_signal_receiver();
    let open_log = OpenLog::get_dart_signal_receiver();
    let log_from_dart = LogFromDart::get_dart_signal_receiver();
    let player_buffering = PlayerBuffering::get_dart_signal_receiver();
    let displays_probe = DisplaysProbe::get_dart_signal_receiver();
    let mpv_versions = MpvVersions::get_dart_signal_receiver();
    let playback_probe = PlaybackProbe::get_dart_signal_receiver();
    let playback_drops = PlaybackDrops::get_dart_signal_receiver();
    let mut buffering = Buffering::default();

    // Playback
    let play = Play::get_dart_signal_receiver();
    let select_quality = SelectQuality::get_dart_signal_receiver();
    let go_to_time = GoToTime::get_dart_signal_receiver();
    let go_to_chapter = GoToChapter::get_dart_signal_receiver();
    let player_position = PlayerPosition::get_dart_signal_receiver();
    let leave_player = LeavePlayer::get_dart_signal_receiver();
    let arm_sleep_timer = ArmSleepTimer::get_dart_signal_receiver();
    let turn_off_sleep_timer = TurnOffSleepTimer::get_dart_signal_receiver();
    let app_backgrounded = AppBackgrounded::get_dart_signal_receiver();
    let app_foregrounded = AppForegrounded::get_dart_signal_receiver();
    let player_started_away = PlayerStartedAway::get_dart_signal_receiver();
    let rewind_stream = RewindStream::get_dart_signal_receiver();
    let back_to_live = BackToLive::get_dart_signal_receiver();
    let refresh_playback = RefreshPlayback::get_dart_signal_receiver();
    // Channel
    let open_channel = OpenChannel::get_dart_signal_receiver();
    let refresh_channel = RefreshChannel::get_dart_signal_receiver();
    let select_vod_kind = SelectVodKind::get_dart_signal_receiver();
    let vods_next_page = VodsNextPage::get_dart_signal_receiver();

    // Chat
    let chat_ready = ChatReady::get_dart_signal_receiver();
    let set_theme = SetTheme::get_dart_signal_receiver();
    let hide_chat = HideChat::get_dart_signal_receiver();
    let show_chat = ShowChat::get_dart_signal_receiver();

    // Search
    let search_typed = SearchTyped::get_dart_signal_receiver();
    let refresh_search = RefreshSearch::get_dart_signal_receiver();
    let next_search_page = NextSearchPage::get_dart_signal_receiver();

    // Settings
    let settings_ready = SettingsReady::get_dart_signal_receiver();
    let change_setting = ChangeSetting::get_dart_signal_receiver();

    // Backup
    let export_backup = ExportBackup::get_dart_signal_receiver();
    let import_backup = ImportBackup::get_dart_signal_receiver();

    loop {
        // Chat: the batch deadline, copied out so its timer arm borrows nothing.
        let chat_flush_at = chat::flush_at(&c.chat);
        // Search: when typing has paused long enough, copied out likewise.
        let search_due_at = search::due_at(&c.search);
        // Playback: the sleep timer's next wake; none once it is off, left
        // or replaced, so no stale tick can reach it.
        let sleep_wake = playback::sleep_wake(&c.playback);
        // Playback: when the player's wait on its stream reaches the stall
        // limit; none unless it waits.
        let stall_wake = playback::stall_wake(&c.playback);
        tokio::select! {
            // Boot: already booted; a hot-restarted Dart side asks again.
            Some(_) = boot.recv() => Booted.send_signal_to_dart(),

            // Browse
            Some(_) = browse_ready.recv() => browse::ready(&c.browse),
            Some(_) = refresh_browse.recv() => browse::refresh(&mut c.browse, &twitch, &fetch),
            Some(_) = next_browse_page.recv() => browse::next(&mut c.browse, &twitch, &fetch),

            // Categories
            Some(_) = categories_ready.recv() => categories::ready(&c.top_categories, &c.favorites),
            Some(_) = refresh_categories.recv() => {
                categories::refresh(&mut c.top_categories, &twitch, &fetch);
            }
            Some(_) = next_categories_page.recv() => {
                categories::next(&mut c.top_categories, &twitch, &fetch);
            }
            Some(pack) = open_category.recv() => {
                let id = &pack.message.id;
                category::open(&mut c.category, &c.favorites, id, &twitch, &fetch);
            }
            Some(_) = refresh_category.recv() => {
                category::refresh(&mut c.category, &c.favorites, &twitch, &fetch);
            }
            Some(_) = next_category_page.recv() => {
                category::next(&mut c.category, &c.favorites, &twitch, &fetch);
            }

            // Favorites: each change also reaches the categories that show them.
            Some(_) = favorites_ready.recv() => favorites::on_ready(&c.favorites, &twitch, &fetch),
            Some(pack) = add_favorite.recv() => favorites::add(&pack.message.login, &twitch, &fetch),
            Some(pack) = remove_favorite.recv() => {
                favorites::remove(&mut c.favorites, &pack.message.channel_id);
                favorites_changed(&c);
            }
            Some(_) = refresh_favorites.recv() => {
                favorites::refresh(&mut c.favorites, &twitch, &fetch);
                favorites_changed(&c);
            }

            // History
            Some(_) = history_ready.recv() => history::ready(&c.history),
            Some(pack) = remove_visit.recv() => history::remove(&mut c.history, pack.message),
            Some(pack) = remove_from_history.recv() => {
                history::remove_target(&mut c.history, pack.message);
            }
            Some(_) = clear_history.recv() => history::clear(&mut c.history),

            // Logs
            Some(_) = logs_ready.recv() => logs::ready(&log),
            Some(pack) = open_log.recv() => logs::open(&log, pack.message),
            Some(pack) = log_from_dart.recv() => logs::from_dart(pack.message),
            Some(pack) = displays_probe.recv() => probes::displays(&pack.message),
            Some(pack) = mpv_versions.recv() => probes::mpv_versions(&pack.message),
            Some(pack) = playback_probe.recv() => probes::playback(&pack.message),
            Some(pack) = playback_drops.recv() => probes::drops(&pack.message),
            Some(pack) = player_buffering.recv() => {
                // Playback: watches the stream for stalls.
                playback::buffering(&mut c.playback, pack.message.buffering);
                logs::buffering(&mut buffering, pack.message, std::time::Instant::now());
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
            Some(pack) = go_to_time.recv() => playback::go_to_time(pack.message),
            Some(pack) = go_to_chapter.recv() => playback::go_to_chapter(&c.playback, pack.message),
            Some(pack) = player_position.recv() => {
                if let Some(at) = playback::position(&mut c.playback, &mut c.resume, pack.message) {
                    // History: counts what the player plays, by the page.
                    history::heard(&mut c.history, &c.playback, &at.target);
                    // Chat: a VOD's replay follows the player, by the stream
                    // it plays (a rewound live page's recording).
                    if let Some(stream) = &at.stream {
                        let (position, duration) = (at.position, at.duration);
                        chat::position(&mut c.chat, &twitch, &fetch, stream, position, duration);
                    }
                }
            }
            Some(pack) = leave_player.recv() => {
                // History: the visit's last time, while the player still has it.
                history::leave(&mut c.history, &c.playback, &pack.message.target);
                // Chat: closes when the player stops.
                if playback::leave(&mut c.playback, &mut c.resume, pack.message) {
                    chat::stop(&mut c.chat, &twitch, &fetch);
                }
            }
            Some(pack) = arm_sleep_timer.recv() => playback::arm_sleep(&mut c.playback, pack.message),
            Some(_) = turn_off_sleep_timer.recv() => playback::turn_off_sleep(&mut c.playback),
            Some(pack) = app_backgrounded.recv() => {
                let audio = c.settings.state().background_audio;
                playback::background(&mut c.playback, audio, pack.message);
            }
            Some(pack) = player_started_away.recv() => {
                let audio = c.settings.state().background_audio;
                playback::play_away(&mut c.playback, audio, pack.message);
            }
            Some(pack) = app_foregrounded.recv() => {
                playback::foreground(&mut c.playback, &c.resume, pack.message);
            }
            Some(pack) = rewind_stream.recv() => {
                playback::rewind(&mut c.playback, &twitch, &fetch, pack.message);
            }
            Some(pack) = back_to_live.recv() => {
                playback::back_to_live(&mut c.playback, &twitch, &fetch, pack.message);
            }
            Some(pack) = refresh_playback.recv() => {
                playback::refresh(&c.playback, &twitch, &fetch, pack.message);
            }
            // The same sleep as the chat batch's: pending while no timer is armed.
            () = chat::until(sleep_wake) => {
                // Chat: closes when the sleep timer stops the player.
                if playback::sleep_tick(&mut c.playback) {
                    chat::stop(&mut c.chat, &twitch, &fetch);
                }
            }

            () = chat::until(stall_wake) => playback::stall_tick(&mut c.playback, &twitch, &fetch),

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
            Some(_) = hide_chat.recv() => chat::hide(&mut c.chat, &twitch, &fetch),
            Some(_) = show_chat.recv() => chat::show(&mut c.chat, &twitch, &fetch),
            () = chat::until(chat_flush_at) => chat::flush(&mut c.chat),

            // Search
            Some(pack) = search_typed.recv() => {
                search::typed(&mut c.search, pack.message.text, Instant::now());
            }
            Some(pack) = refresh_search.recv() => {
                search::refresh(&mut c.search, pack.message.list, &twitch, &fetch);
            }
            Some(pack) = next_search_page.recv() => {
                search::next(&mut c.search, pack.message.list, &twitch, &fetch);
            }
            // The same sleep as the chat batch's: pending while nothing waits.
            () = chat::until(search_due_at) => search::settle(&mut c.search, &twitch, &fetch),

            // Settings: the chat takes a change at once; playback reads the
            // settings on the next `Play`.
            Some(_) = settings_ready.recv() => settings::ready(&c.settings),
            Some(pack) = change_setting.recv() => {
                if settings::change(&mut c.settings, pack.message) {
                    chat::settings_changed(&mut c.chat, *c.settings.state());
                }
            }

            // Backup: an import reaches the favorites, the history and the
            // settings, so each reloads.
            Some(_) = export_backup.recv() => backup::export(&c.store),
            Some(pack) = import_backup.recv() => {
                if backup::import(&mut c.store, &pack.message) {
                    imported(&mut c, &twitch, &fetch);
                }
            }

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
        // Categories
        Fetched::TopCategories { request, result } => {
            categories::on_page(&mut c.top_categories, request, result);
        }
        Fetched::CategoryStreams { request, result } => {
            category::on_page(&mut c.category, &c.favorites, request, result);
        }
        // Favorites
        Fetched::Favorites(fetched) => {
            favorites::on_fetched(&mut c.favorites, fetched, twitch, fetch);
            favorites_changed(c);
        }
        // Playback
        Fetched::Playback {
            target,
            result,
            details,
        } => {
            let preferred = c.settings.state().start_quality;
            playback::resolved(
                &mut c.playback,
                &c.resume,
                target,
                result,
                details,
                preferred,
            );
        }
        Fetched::PlaybackTimedOut(attempt) => playback::timed_out(&mut c.playback, attempt),
        Fetched::PlaybackDetails {
            target,
            stream,
            details,
        } => playback::refreshed(&mut c.playback, &target, &stream, details),
        Fetched::Rewound {
            target,
            video,
            result,
            details,
        } => {
            let preferred = c.settings.state().start_quality;
            let stream = playback::rewound(
                &mut c.playback,
                &c.resume,
                &target,
                &video,
                result,
                details,
                preferred,
            );
            // Chat: replays the recording, the panel shown or hidden as it was.
            if let Some(stream) = stream {
                chat::switch(&mut c.chat, twitch, fetch, &stream);
            }
        }
        Fetched::Reloaded { stream, result } => {
            let preferred = c.settings.state().start_quality;
            playback::reloaded(&mut c.playback, &c.resume, &stream, result, preferred);
        }
        Fetched::Returned {
            target,
            video,
            result,
            details,
        } => {
            let preferred = c.settings.state().start_quality;
            let stream = playback::returned(
                &mut c.playback,
                &c.resume,
                &target,
                &video,
                result,
                details,
                preferred,
            );
            // Chat: back to the channel's live chat.
            if let Some(stream) = stream {
                chat::switch(&mut c.chat, twitch, fetch, &stream);
            }
        }
        // Channel
        Fetched::Channel(fetched) => {
            channel::on_fetched(&mut c.channel, &c.resume, fetched, twitch, fetch);
        }
        // Chat
        Fetched::Chat(fetched) => chat::on_fetched(&mut c.chat, twitch, fetch, fetched),
        // Search
        Fetched::Search(fetched) => search::on_fetched(&mut c.search, fetched),
    }
}

/// The favorites list may have changed: the categories strip and the open
/// category's pinned streams are drawn from it, so they go out again.
fn favorites_changed(c: &Controllers) {
    categories::send_from_your_channels(&c.favorites);
    category::favorites_changed(&c.category, &c.favorites);
}

/// A backup was imported into the store: every feature it reaches reloads
/// and goes out again, the chat takes the settings at once, and the
/// favorites' live status is checked.
fn imported(c: &mut Controllers, twitch: &Twitch, fetch: &mpsc::UnboundedSender<Fetched>) {
    favorites::refresh(&mut c.favorites, twitch, fetch);
    favorites_changed(c);
    history::reload(&mut c.history);
    settings::reload(&mut c.settings);
    chat::settings_changed(&mut c.chat, *c.settings.state());
}
