//! Favorite channels kept locally, and which of them are live. The store is
//! written through before state changes; live status comes from the heads,
//! which fetch it for `ids()` and hand it to `apply_live`.

use std::cmp::Reverse;
use std::collections::HashMap;

use crate::models::{Channel, ChannelId, ChannelLogin, Favorite, Stream, Timestamp};
use crate::ports::{FavoritesStore, StoreError};

/// Whether a favorite is live, as of the last `apply_live`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Liveness {
    /// Not checked since it was loaded or added.
    Unknown,
    Offline,
    Live(Box<Stream>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FavoriteEntry {
    pub favorite: Favorite,
    pub liveness: Liveness,
}

impl FavoriteEntry {
    fn viewers(&self) -> Option<u32> {
        match &self.liveness {
            Liveness::Live(stream) => Some(stream.viewers),
            Liveness::Unknown | Liveness::Offline => None,
        }
    }
}

/// The favorites split by what the last live check found, each part in
/// list order. A favorite is in exactly one part.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct FavoritesSplit<'a> {
    /// Live, by viewers descending.
    pub live: Vec<&'a Stream>,
    /// Checked and offline, by name.
    pub offline: Vec<&'a Favorite>,
    /// Not checked since it was loaded or added, by name: neither live nor
    /// known to be offline.
    pub unchecked: Vec<&'a Favorite>,
}

/// Owner of the favorites list. The list is ordered: live channels by
/// viewers descending, then the others by name.
pub struct FavoritesController {
    entries: Vec<FavoriteEntry>,
    store: Box<dyn FavoritesStore>,
}

impl FavoritesController {
    /// Starts empty; call [`FavoritesController::load`] to read the store.
    pub fn new(store: Box<dyn FavoritesStore>) -> Self {
        Self {
            entries: Vec::new(),
            store,
        }
    }

    /// Replaces the list with the stored favorites. Safe to call again to
    /// pick up other processes' writes; a channel still listed keeps its
    /// live status.
    pub fn load(&mut self) -> Result<(), StoreError> {
        let stored = self.store.load_all()?;
        let mut previous: HashMap<ChannelId, Liveness> = self
            .entries
            .drain(..)
            .map(|e| (e.favorite.channel_id, e.liveness))
            .collect();
        self.entries = stored
            .into_iter()
            .map(|favorite| FavoriteEntry {
                liveness: previous
                    .remove(&favorite.channel_id)
                    .unwrap_or(Liveness::Unknown),
                favorite,
            })
            .collect();
        self.sort();
        Ok(())
    }

    pub fn entries(&self) -> &[FavoriteEntry] {
        &self.entries
    }

    /// The list split into live, offline and not yet checked.
    pub fn split(&self) -> FavoritesSplit<'_> {
        let mut split = FavoritesSplit::default();
        for entry in &self.entries {
            match &entry.liveness {
                Liveness::Live(stream) => split.live.push(stream),
                Liveness::Offline => split.offline.push(&entry.favorite),
                Liveness::Unknown => split.unchecked.push(&entry.favorite),
            }
        }
        split
    }

    /// Adds `channel`. Idempotent on its id: a channel already listed keeps
    /// its entry. A failed write leaves the list unchanged.
    pub fn add(&mut self, channel: Channel, now: Timestamp) -> Result<(), StoreError> {
        let favorite = Favorite {
            channel_id: channel.id,
            login: channel.login,
            display_name: channel.display_name,
            added_at: now,
        };
        self.store.add(&favorite)?;
        if !self
            .entries
            .iter()
            .any(|e| e.favorite.channel_id == favorite.channel_id)
        {
            self.entries.push(FavoriteEntry {
                favorite,
                liveness: Liveness::Unknown,
            });
            self.sort();
        }
        Ok(())
    }

    /// Removes the channel `id`. A failed write leaves the list unchanged.
    pub fn remove(&mut self, id: &ChannelId) -> Result<(), StoreError> {
        self.store.remove(id)?;
        self.entries.retain(|e| e.favorite.channel_id != *id);
        Ok(())
    }

    /// The favorite whose cached login is `login`; needs no network.
    pub fn find_by_login(&self, login: &ChannelLogin) -> Option<ChannelId> {
        self.entries
            .iter()
            .find(|e| e.favorite.login == *login)
            .map(|e| e.favorite.channel_id.clone())
    }

    /// The ids whose live status the head should fetch.
    pub fn ids(&self) -> Vec<ChannelId> {
        self.entries
            .iter()
            .map(|e| e.favorite.channel_id.clone())
            .collect()
    }

    /// Applies the live streams fetched for `ids()`: every favorite is
    /// marked live or offline. A stream whose channel is not (or no longer)
    /// a favorite is ignored, so a favorite removed while the fetch was in
    /// flight does not come back.
    ///
    /// A live channel whose login or display name changed is renamed in the
    /// store. Live status is applied even if a rename fails; that favorite
    /// keeps its old name and the first failure is returned.
    pub fn apply_live(&mut self, streams: Vec<Stream>) -> Result<(), StoreError> {
        let mut live: HashMap<ChannelId, Stream> = streams
            .into_iter()
            .map(|s| (s.channel.id.clone(), s))
            .collect();
        let mut result = Ok(());
        for entry in &mut self.entries {
            let Some(stream) = live.remove(&entry.favorite.channel_id) else {
                entry.liveness = Liveness::Offline;
                continue;
            };
            let favorite = &mut entry.favorite;
            let channel = &stream.channel;
            if favorite.login != channel.login || favorite.display_name != channel.display_name {
                match self
                    .store
                    .rename(&channel.id, &channel.login, &channel.display_name)
                {
                    Ok(()) => {
                        favorite.login = channel.login.clone();
                        favorite.display_name.clone_from(&channel.display_name);
                    }
                    Err(e) => {
                        if result.is_ok() {
                            result = Err(e);
                        }
                    }
                }
            }
            entry.liveness = Liveness::Live(Box::new(stream));
        }
        self.sort();
        result
    }

    /// Live by viewers descending, then the rest by name. Ties fall back to
    /// the name, then the login, so the order is total.
    fn sort(&mut self) {
        self.entries.sort_by_cached_key(|e| {
            (
                e.viewers().is_none(),
                Reverse(e.viewers()),
                e.favorite.display_name.to_lowercase(),
                e.favorite.login.as_str().to_owned(),
            )
        });
    }
}
