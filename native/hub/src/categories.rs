//! The Categories page: the top categories controller's actions and its
//! state ↔ signals, and the strip of categories the live favorites are
//! streaming.
//!
//! The top categories go out like the top streams: a loaded list that is
//! still loaded goes out as an `Appended` of its new categories, anything
//! else as a `Snapshot`. The strip goes out whole whenever the favorites
//! may have changed.

use jitter_core::categories::{TopCategoriesController, favorites_by_category};
use jitter_core::favorites::FavoritesController;
use jitter_core::models::{
    Category, FavoritesCategory, FirstPage, ListState, MorePages, Page, PageRequest,
};
use jitter_twitch::Twitch;
use rinf::RustSignal;
use tokio::sync::mpsc::UnboundedSender;

use crate::format::{twitch_error, viewers_label};
use crate::run::Fetched;
use crate::signals::{
    CategoriesMore, CategoriesState, CategoriesUpdate, CategoryTile, FavoritesCategoryRow,
};

/// Box art size requested for a tile: 3:4.
const BOX_ART_WIDTH: u32 = 285;
const BOX_ART_HEIGHT: u32 = 380;

/// `CategoriesReady`: the whole state, for a holder that just subscribed.
pub fn ready(top: &TopCategoriesController, favorites: &FavoritesController) {
    CategoriesUpdate::since(None, top.state()).send_signal_to_dart();
    send_from_your_channels(favorites);
}

/// `RefreshCategories`, and the first load at start: starts over from the
/// first page and fetches it.
pub fn refresh(
    top: &mut TopCategoriesController,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
) {
    let shown = CategoriesUpdate::shown(top.state());
    let request = top.begin_refresh();
    spawn_fetch(request, twitch, fetch);
    CategoriesUpdate::since(shown, top.state()).send_signal_to_dart();
}

/// `NextCategoriesPage`: fetches the next page, unless core says there is
/// none to start now.
pub fn next(top: &mut TopCategoriesController, twitch: &Twitch, fetch: &UnboundedSender<Fetched>) {
    let shown = CategoriesUpdate::shown(top.state());
    if let Some(request) = top.begin_next() {
        spawn_fetch(request, twitch, fetch);
        CategoriesUpdate::since(shown, top.state()).send_signal_to_dart();
    }
}

/// A fetched page, handed to core; core ignores a stale one.
pub fn on_page(
    top: &mut TopCategoriesController,
    request: PageRequest,
    result: Result<Page<Category>, String>,
) {
    let shown = CategoriesUpdate::shown(top.state());
    top.apply_page(request, result);
    CategoriesUpdate::since(shown, top.state()).send_signal_to_dart();
}

/// The strip, as core groups the favorites now.
pub fn send_from_your_channels(favorites: &FavoritesController) {
    CategoriesUpdate::FromYourChannels {
        categories: favorites_by_category(favorites.entries())
            .iter()
            .map(FavoritesCategoryRow::from)
            .collect(),
    }
    .send_signal_to_dart();
}

fn spawn_fetch(request: PageRequest, twitch: &Twitch, fetch: &UnboundedSender<Fetched>) {
    let (twitch, fetch) = (twitch.clone(), fetch.clone());
    tokio::spawn(async move {
        let result = twitch
            .top_categories(request.cursor.as_ref())
            .await
            .map_err(|e| twitch_error(&e));
        let _ = fetch.send(Fetched::TopCategories { request, result });
    });
}

impl CategoriesUpdate {
    /// How many categories Dart lists after the last update, when it lists
    /// a loaded page.
    fn shown(state: &ListState<Category>) -> Option<usize> {
        match state {
            ListState::Loaded { items, .. } => Some(items.len()),
            ListState::Idle | ListState::FirstPage(_) => None,
        }
    }

    /// The update that takes Dart from `shown` categories to `state`.
    fn since(shown: Option<usize>, state: &ListState<Category>) -> Self {
        match (shown, state) {
            (Some(shown), ListState::Loaded { items, more, .. }) => Self::Appended {
                categories: tiles(items.get(shown..).unwrap_or_default()),
                more: more.into(),
            },
            _ => Self::Snapshot {
                state: state.into(),
            },
        }
    }
}

impl From<&ListState<Category>> for CategoriesState {
    fn from(state: &ListState<Category>) -> Self {
        match state {
            ListState::Idle => Self::Idle,
            ListState::FirstPage(FirstPage::Loading) => Self::Loading,
            ListState::FirstPage(FirstPage::Failed { message }) => Self::Failed {
                message: message.clone(),
            },
            ListState::Loaded { items, more, .. } => Self::Loaded {
                categories: tiles(items),
                more: more.into(),
            },
        }
    }
}

impl From<&MorePages> for CategoriesMore {
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

fn tiles(categories: &[Category]) -> Vec<CategoryTile> {
    categories.iter().map(CategoryTile::from).collect()
}

impl From<&Category> for CategoryTile {
    fn from(category: &Category) -> Self {
        Self {
            id: category.id.as_str().to_owned(),
            name: category.name.clone(),
            box_art_url: category.box_art(BOX_ART_WIDTH, BOX_ART_HEIGHT),
            viewers_label: viewers_label(category.viewers),
        }
    }
}

impl From<&FavoritesCategory<'_>> for FavoritesCategoryRow {
    fn from(category: &FavoritesCategory<'_>) -> Self {
        Self {
            id: category.game.id.as_str().to_owned(),
            name: category.game.name.clone(),
            channels: category
                .streams
                .iter()
                .map(|s| s.channel.display_name.clone())
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use jitter_core::models::{CategoryId, Cursor};

    use super::*;

    type TestResult = Result<(), String>;

    fn category(id: &str) -> Result<Category, String> {
        Ok(Category {
            id: CategoryId::parse(id).map_err(|e| format!("{e:?}"))?,
            name: format!("Game {id}"),
            box_art_url: format!("https://b/{id}-144x192.jpg"),
            viewers: 45_678,
        })
    }

    fn page(ids: &[&str], next: Option<&str>) -> Result<Page<Category>, String> {
        Ok(Page {
            items: ids
                .iter()
                .map(|id| category(id))
                .collect::<Result<_, _>>()?,
            next: next.map(Cursor::new),
        })
    }

    #[test]
    fn a_tile_is_ready_to_show() -> TestResult {
        let tile = CategoryTile::from(&category("21779")?);
        assert_eq!(tile.id, "21779");
        assert_eq!(tile.box_art_url, "https://b/21779-285x380.jpg");
        assert_eq!(tile.viewers_label, "45.6K viewers");
        Ok(())
    }

    #[test]
    fn a_next_page_appends_only_its_new_categories() -> TestResult {
        let mut top = TopCategoriesController::new();
        let request = top.begin_refresh();
        top.apply_page(request, Ok(page(&["1", "2"], Some("c"))?));
        let request = top.begin_next().ok_or("no next page")?;
        let shown = CategoriesUpdate::shown(top.state());
        top.apply_page(request, Ok(page(&["2", "3"], None)?));
        let CategoriesUpdate::Appended { categories, more } =
            CategoriesUpdate::since(shown, top.state())
        else {
            return Err("not appended".into());
        };
        let ids: Vec<&str> = categories.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["3"]);
        assert!(matches!(more, CategoriesMore::Idle));
        Ok(())
    }
}
