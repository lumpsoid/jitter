//! One module per command, plus what they share: the error type, opening
//! the shared database and parsing arguments into core's value objects.

pub mod categories;
pub mod category;
pub mod chat;
pub mod fav;
pub mod history;
pub mod play;
pub mod replay;
pub mod resume;
pub mod search;
pub mod settings;
pub mod top;
pub mod vods;

use std::fmt::Display;
use std::time::{SystemTime, UNIX_EPOCH};

use jitter_core::favorites::FavoritesController;
use jitter_core::models::{ChannelLogin, Settings, Timestamp, VideoId};
use jitter_core::settings::SettingsController;
use jitter_store::SqliteStore;
use jitter_twitch::Twitch;

/// A message for the user; `main` prints it and exits non-zero.
pub type Result<T> = std::result::Result<T, String>;

/// Adds the failing step to an error: `"looking up xqc: not found"`.
pub fn ctx<E: Display>(what: impl Display) -> impl FnOnce(E) -> String {
    move |e| format!("{what}: {e}")
}

/// The database every head on this machine shares.
pub fn open_store() -> Result<SqliteStore> {
    let path = jitter_store::default_db_path().map_err(ctx("locating the database"))?;
    SqliteStore::open(path).map_err(ctx("opening the database"))
}

/// The user's stored settings, the same the GUI reads.
pub fn settings() -> Result<Settings> {
    let mut settings = SettingsController::new(Box::new(open_store()?));
    settings.load().map_err(ctx("loading the settings"))?;
    Ok(*settings.state())
}

pub fn twitch() -> Result<Twitch> {
    Twitch::new().map_err(ctx("starting the Twitch client"))
}

/// The stored favorites, with which of them are live checked now.
pub async fn live_favorites(twitch: &Twitch) -> Result<FavoritesController> {
    let mut favorites = FavoritesController::new(Box::new(open_store()?));
    favorites.load().map_err(ctx("loading favorites"))?;
    let streams = twitch
        .live_streams(&favorites.ids())
        .await
        .map_err(ctx("checking live status"))?;
    favorites
        .apply_live(streams)
        .map_err(ctx("updating favorites"))?;
    Ok(favorites)
}

pub fn login(input: &str) -> Result<ChannelLogin> {
    ChannelLogin::parse(input).map_err(|e| format!("invalid channel login {input:?}: {e:?}"))
}

pub fn video_id(input: &str) -> Result<VideoId> {
    VideoId::parse(input).map_err(|e| format!("invalid video id {input:?}: {e:?}"))
}

/// Heads pass the clock to core; core never reads it.
pub fn now() -> Result<Timestamp> {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(ctx("reading the clock"))?;
    let seconds = i64::try_from(since_epoch.as_secs()).map_err(ctx("reading the clock"))?;
    Ok(Timestamp::from_unix(seconds))
}
