use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::models::{ChannelId, ChannelLogin, Favorite};
use crate::ports::{FavoritesStore, StoreError};

/// In-memory [`FavoritesStore`] that keeps the port's contract: `add` never
/// overwrites, `rename` never inserts. Can be told to fail loads or writes,
/// and records which ids were passed to `rename`.
#[derive(Debug, Clone, Default)]
pub struct FakeFavoritesStore {
    inner: Arc<Mutex<Inner>>,
    fail_loads: bool,
    fail_writes: bool,
}

#[derive(Debug, Default)]
struct Inner {
    favorites: Vec<Favorite>,
    renamed: Vec<ChannelId>,
}

impl FakeFavoritesStore {
    /// A store already holding `favorites`.
    pub fn with_favorites(favorites: Vec<Favorite>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                favorites,
                renamed: Vec::new(),
            })),
            ..Self::default()
        }
    }

    /// Every `load_all` returns `Err`.
    pub fn failing_loads(self) -> Self {
        Self {
            fail_loads: true,
            ..self
        }
    }

    /// Every `add`, `rename` and `remove` returns `Err` and stores nothing.
    pub fn failing_writes(self) -> Self {
        Self {
            fail_writes: true,
            ..self
        }
    }

    /// What is stored now.
    pub fn favorites(&self) -> Vec<Favorite> {
        self.lock().favorites.clone()
    }

    /// The ids passed to `rename`, in call order, including failed calls.
    pub fn renamed(&self) -> Vec<ChannelId> {
        self.lock().renamed.clone()
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn check_write(&self) -> Result<(), StoreError> {
        if self.fail_writes {
            return Err(StoreError::new("fake write failure"));
        }
        Ok(())
    }
}

impl FavoritesStore for FakeFavoritesStore {
    fn load_all(&self) -> Result<Vec<Favorite>, StoreError> {
        if self.fail_loads {
            return Err(StoreError::new("fake load failure"));
        }
        Ok(self.favorites())
    }

    fn add(&mut self, favorite: &Favorite) -> Result<(), StoreError> {
        self.check_write()?;
        let mut inner = self.lock();
        if !inner
            .favorites
            .iter()
            .any(|f| f.channel_id == favorite.channel_id)
        {
            inner.favorites.push(favorite.clone());
        }
        Ok(())
    }

    fn rename(
        &mut self,
        id: &ChannelId,
        login: &ChannelLogin,
        display_name: &str,
    ) -> Result<(), StoreError> {
        self.lock().renamed.push(id.clone());
        self.check_write()?;
        if let Some(stored) = self
            .lock()
            .favorites
            .iter_mut()
            .find(|f| f.channel_id == *id)
        {
            stored.login = login.clone();
            display_name.clone_into(&mut stored.display_name);
        }
        Ok(())
    }

    fn remove(&mut self, id: &ChannelId) -> Result<(), StoreError> {
        self.check_write()?;
        self.lock().favorites.retain(|f| f.channel_id != *id);
        Ok(())
    }
}
