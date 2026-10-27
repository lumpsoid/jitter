//! Favorites controller ↔ favorites signals, and the calls the favorites
//! need: a channel lookup to add one, and the live check. The list is sent
//! whole after every action; a failure goes out first as a one-shot
//! `Failed`. Live status is checked when the holder connects and on a
//! manual refresh only, never on a timer.

use std::fmt::Display;
use std::time::{SystemTime, UNIX_EPOCH};

use jitter_core::favorites::{FavoriteEntry, FavoritesController, Liveness};
use jitter_core::models::{Channel, ChannelId, ChannelLogin, Stream, Timestamp};
use jitter_twitch::{Twitch, TwitchError};
use rinf::RustSignal;
use tokio::sync::mpsc::UnboundedSender;

use crate::format::{twitch_error, viewers_label};
use crate::run::Fetched;
use crate::signals::{FavoriteRow, FavoriteStatus, FavoritesUpdate};

/// A favorites call's result, back in the loop as [`Fetched::Favorites`].
pub enum FavoritesFetched {
    /// The lookup an `AddFavorite` started.
    Channel {
        login: ChannelLogin,
        result: Result<Channel, TwitchError>,
    },
    /// The live check of every favorite.
    Live(Result<Vec<Stream>, TwitchError>),
}

/// `FavoritesReady`: the list as it stands, then a live check.
pub fn on_ready(
    favorites: &FavoritesController,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
) {
    send_snapshot(favorites);
    check_live(favorites, twitch, fetch);
}

/// `RefreshFavorites`: re-reads the store, so another head's changes show
/// up, then checks which favorites are live.
pub fn refresh(
    favorites: &mut FavoritesController,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
) {
    report(favorites.load(), "Couldn't reload favorites");
    send_snapshot(favorites);
    check_live(favorites, twitch, fetch);
}

/// `AddFavorite`: looks the channel up; it is added when the lookup comes
/// back, in [`on_fetched`].
pub fn add(login: &str, twitch: &Twitch, fetch: &UnboundedSender<Fetched>) {
    let login = match ChannelLogin::parse(login) {
        Ok(login) => login,
        Err(e) => {
            return send_failed(format!(
                "Couldn't add the favorite: invalid channel login {login:?}: {e:?}"
            ));
        }
    };
    let (twitch, fetch) = (twitch.clone(), fetch.clone());
    tokio::spawn(async move {
        let result = twitch.channel(&login).await;
        let fetched = FavoritesFetched::Channel { login, result };
        let _ = fetch.send(Fetched::Favorites(fetched));
    });
}

/// `RemoveFavorite`.
pub fn remove(favorites: &mut FavoritesController, channel_id: &str) {
    let removed = ChannelId::parse(channel_id)
        .map_err(|e| format!("invalid channel id {channel_id:?}: {e:?}"))
        .and_then(|id| favorites.remove(&id).map_err(|e| e.to_string()));
    report(removed, "Couldn't remove the favorite");
    send_snapshot(favorites);
}

/// Hands a favorites call's result to the controller, then sends the list.
pub fn on_fetched(favorites: &mut FavoritesController, fetched: FavoritesFetched) {
    match fetched {
        FavoritesFetched::Channel { login, result } => {
            report(
                add_found(favorites, result),
                &format!("Couldn't add {login}"),
            );
        }
        FavoritesFetched::Live(Ok(streams)) => {
            report(favorites.apply_live(streams), "Couldn't rename a favorite");
        }
        FavoritesFetched::Live(Err(e)) => {
            send_failed(format!("Couldn't check who is live: {}", twitch_error(&e)));
        }
    }
    send_snapshot(favorites);
}

fn add_found(
    favorites: &mut FavoritesController,
    found: Result<Channel, TwitchError>,
) -> Result<(), String> {
    let channel = found.map_err(|e| twitch_error(&e))?;
    favorites.add(channel, now()?).map_err(|e| e.to_string())
}

/// Spawns the live check of every favorite listed now.
fn check_live(favorites: &FavoritesController, twitch: &Twitch, fetch: &UnboundedSender<Fetched>) {
    let ids = favorites.ids();
    let (twitch, fetch) = (twitch.clone(), fetch.clone());
    tokio::spawn(async move {
        let result = twitch.live_streams(&ids).await;
        let _ = fetch.send(Fetched::Favorites(FavoritesFetched::Live(result)));
    });
}

/// Heads pass the clock to core; core never reads it.
fn now() -> Result<Timestamp, String> {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("reading the clock: {e}"))?;
    let seconds =
        i64::try_from(since_epoch.as_secs()).map_err(|e| format!("reading the clock: {e}"))?;
    Ok(Timestamp::from_unix(seconds))
}

/// Sends a failed action's error as `Failed`, prefixed with `what`.
fn report<E: Display>(result: Result<(), E>, what: &str) {
    if let Err(e) = result {
        send_failed(format!("{what}: {e}"));
    }
}

fn send_failed(message: String) {
    FavoritesUpdate::Failed { message }.send_signal_to_dart();
}

fn send_snapshot(favorites: &FavoritesController) {
    FavoritesUpdate::Snapshot {
        favorites: favorites.entries().iter().map(FavoriteRow::from).collect(),
    }
    .send_signal_to_dart();
}

impl From<&FavoriteEntry> for FavoriteRow {
    fn from(entry: &FavoriteEntry) -> Self {
        let favorite = &entry.favorite;
        let (avatar_url, status) = match &entry.liveness {
            Liveness::Unknown => (None, FavoriteStatus::Unknown),
            Liveness::Offline => (None, FavoriteStatus::Offline),
            Liveness::Live(stream) => (
                Some(stream.channel.avatar_url.clone()),
                FavoriteStatus::Live {
                    viewers_label: viewers_label(stream.viewers),
                    title: stream.title.clone(),
                },
            ),
        };
        Self {
            channel_id: favorite.channel_id.as_str().to_owned(),
            login: favorite.login.as_str().to_owned(),
            display_name: favorite.display_name.clone(),
            avatar_url,
            status,
        }
    }
}
