//! One category's page: its streams controller, the call it needs, and its
//! state ↔ signals, with the favorites core pins first.
//!
//! The streams go out like the top streams: a loaded list that is still
//! loaded goes out as an `Appended` of its new unpinned streams, anything
//! else as a `Snapshot`. The pinned favorites change only with the
//! favorites, which resend the whole page ([`favorites_changed`]), so
//! between two snapshots the unpinned list only grows.

use jitter_core::category::{CategoryController, pin_favorites};
use jitter_core::favorites::FavoritesController;
use jitter_core::models::{CategoryId, FirstPage, ListState, MorePages, Page, PageRequest, Stream};
use jitter_twitch::Twitch;
use rinf::RustSignal;
use tokio::sync::mpsc::UnboundedSender;

use crate::format::twitch_error;
use crate::run::Fetched;
use crate::signals::{CategoryMore, CategoryState, CategoryUpdate, StreamCard};

/// The open category's controller; made by the first `OpenCategory`, as
/// it needs a category.
pub type CategoryPage = Option<CategoryController>;

/// `OpenCategory`: lists category `id`'s streams from the first page. An
/// id that cannot be a category's closes the page's controller, so a
/// page still in flight for the one before is dropped.
pub fn open(
    page: &mut CategoryPage,
    favorites: &FavoritesController,
    id: &str,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
) {
    let Ok(id) = CategoryId::parse(id) else {
        *page = None;
        let message = format!("{id:?} is not a category id.");
        let state = CategoryState::Failed { message };
        return CategoryUpdate::Snapshot { state }.send_signal_to_dart();
    };
    let category = page.get_or_insert_with(|| CategoryController::new(id.clone()));
    let request = category.show(id);
    spawn_fetch(category, request, twitch, fetch);
    CategoryUpdate::since(None, category, favorites).send_signal_to_dart();
}

/// `RefreshCategory`: starts the open category over from the first page.
pub fn refresh(
    page: &mut CategoryPage,
    favorites: &FavoritesController,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
) {
    if let Some(category) = page.as_mut() {
        let request = category.begin_refresh();
        spawn_fetch(category, request, twitch, fetch);
        CategoryUpdate::since(None, category, favorites).send_signal_to_dart();
    }
}

/// `NextCategoryPage`: fetches the next page, unless core says there is
/// none to start now.
pub fn next(
    page: &mut CategoryPage,
    favorites: &FavoritesController,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
) {
    let Some(category) = page.as_mut() else {
        return;
    };
    let shown = CategoryUpdate::shown(category, favorites);
    if let Some(request) = category.begin_next() {
        spawn_fetch(category, request, twitch, fetch);
        CategoryUpdate::since(shown, category, favorites).send_signal_to_dart();
    }
}

/// A fetched page, handed to core; core ignores a stale one.
pub fn on_page(
    page: &mut CategoryPage,
    favorites: &FavoritesController,
    request: PageRequest,
    result: Result<Page<Stream>, String>,
) {
    if let Some(category) = page.as_mut() {
        let shown = CategoryUpdate::shown(category, favorites);
        category.apply_page(request, result);
        CategoryUpdate::since(shown, category, favorites).send_signal_to_dart();
    }
}

/// The favorites may have changed, and with them who is pinned: the whole
/// page again.
pub fn favorites_changed(page: &CategoryPage, favorites: &FavoritesController) {
    if let Some(category) = page {
        CategoryUpdate::since(None, category, favorites).send_signal_to_dart();
    }
}

/// Fetches `request` for the category `category` lists now.
fn spawn_fetch(
    category: &CategoryController,
    request: PageRequest,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
) {
    let id = category.id().clone();
    let (twitch, fetch) = (twitch.clone(), fetch.clone());
    tokio::spawn(async move {
        let result = twitch
            .category_streams(&id, request.cursor.as_ref())
            .await
            .map_err(|e| twitch_error(&e));
        let _ = fetch.send(Fetched::CategoryStreams { request, result });
    });
}

impl CategoryUpdate {
    /// How many unpinned streams Dart lists after the last update, when it
    /// lists a loaded page.
    fn shown(category: &CategoryController, favorites: &FavoritesController) -> Option<usize> {
        match category.state() {
            ListState::Loaded { items, .. } => Some(
                pin_favorites(category.id(), favorites.entries(), items)
                    .others
                    .len(),
            ),
            ListState::Idle | ListState::FirstPage(_) => None,
        }
    }

    /// The update that takes Dart from `shown` unpinned streams to the
    /// category's state.
    fn since(
        shown: Option<usize>,
        category: &CategoryController,
        favorites: &FavoritesController,
    ) -> Self {
        let state = match category.state() {
            ListState::Idle => CategoryState::Idle,
            ListState::FirstPage(FirstPage::Loading) => CategoryState::Loading,
            ListState::FirstPage(FirstPage::Failed { message }) => CategoryState::Failed {
                message: message.clone(),
            },
            ListState::Loaded { items, more, .. } => {
                let pinned = pin_favorites(category.id(), favorites.entries(), items);
                if let Some(shown) = shown {
                    return Self::Appended {
                        streams: cards(pinned.others.get(shown..).unwrap_or_default()),
                        more: more.into(),
                    };
                }
                CategoryState::Loaded {
                    favorites: cards(&pinned.favorites),
                    streams: cards(&pinned.others),
                    more: more.into(),
                }
            }
        };
        Self::Snapshot { state }
    }
}

impl From<&MorePages> for CategoryMore {
    fn from(more: &MorePages) -> Self {
        match more {
            MorePages::Idle => Self::Idle,
            MorePages::Loading => Self::Loading,
            MorePages::Failed { message } => Self::Failed {
                message: message.clone(),
            },
        }
    }
}

fn cards(streams: &[&Stream]) -> Vec<StreamCard> {
    streams.iter().map(|s| StreamCard::from(*s)).collect()
}

#[cfg(test)]
mod tests {
    use jitter_core::fakes::FakeFavoritesStore;
    use jitter_core::models::Timestamp;
    use jitter_core::models::{Channel, ChannelId, ChannelLogin, Cursor, Favorite, GameRef};

    use super::*;

    type TestResult = Result<(), String>;

    fn category_id() -> Result<CategoryId, String> {
        CategoryId::parse("512953").map_err(|e| format!("{e:?}"))
    }

    fn channel(id: &str, login: &str) -> Result<Channel, String> {
        Ok(Channel {
            id: ChannelId::parse(id).map_err(|e| format!("{e:?}"))?,
            login: ChannelLogin::parse(login).map_err(|e| format!("{e:?}"))?,
            display_name: login.to_owned(),
            avatar_url: String::new(),
        })
    }

    fn stream(id: &str, login: &str) -> Result<Stream, String> {
        Ok(Stream {
            id: format!("s{id}"),
            channel: channel(id, login)?,
            title: String::new(),
            game: Some(GameRef {
                id: category_id()?,
                name: "Elden Ring".to_owned(),
            }),
            viewers: 1,
            started_at: None,
            thumbnail_url: String::new(),
        })
    }

    fn page(streams: &[(&str, &str)], next: Option<&str>) -> Result<Page<Stream>, String> {
        Ok(Page {
            items: streams
                .iter()
                .map(|(id, login)| stream(id, login))
                .collect::<Result<_, _>>()?,
            next: next.map(Cursor::new),
        })
    }

    /// The favorites, with `vinesauce` (id 3) live in the category.
    fn favorites() -> Result<FavoritesController, String> {
        let vinesauce = channel("3", "vinesauce")?;
        let store = FakeFavoritesStore::with_favorites(vec![Favorite {
            channel_id: vinesauce.id,
            login: vinesauce.login,
            display_name: vinesauce.display_name,
            added_at: Timestamp::from_unix(0),
        }]);
        let mut favorites = FavoritesController::new(Box::new(store));
        favorites.load().map_err(|e| e.to_string())?;
        favorites
            .apply_live(vec![stream("3", "vinesauce")?])
            .map_err(|e| e.to_string())?;
        Ok(favorites)
    }

    fn logins(cards: &[StreamCard]) -> Vec<&str> {
        cards.iter().map(|c| c.login.as_str()).collect()
    }

    #[test]
    fn the_first_page_pins_the_favorites_and_later_pages_append_the_rest() -> TestResult {
        let favorites = favorites()?;
        let mut category = CategoryController::new(category_id()?);
        let request = category.begin_refresh();
        category.apply_page(
            request,
            Ok(page(&[("10", "biggest"), ("3", "vinesauce")], Some("c"))?),
        );
        let CategoryUpdate::Snapshot {
            state:
                CategoryState::Loaded {
                    favorites: pinned,
                    streams,
                    ..
                },
        } = CategoryUpdate::since(None, &category, &favorites)
        else {
            return Err("not a loaded snapshot".into());
        };
        assert_eq!(logins(&pinned), ["vinesauce"]);
        assert_eq!(logins(&streams), ["biggest"]);

        let request = category.begin_next().ok_or("no next page")?;
        let shown = CategoryUpdate::shown(&category, &favorites);
        category.apply_page(request, Ok(page(&[("11", "other")], None)?));
        let CategoryUpdate::Appended { streams, more } =
            CategoryUpdate::since(shown, &category, &favorites)
        else {
            return Err("not appended".into());
        };
        assert_eq!(logins(&streams), ["other"]);
        assert!(matches!(more, CategoryMore::Idle));
        Ok(())
    }
}
