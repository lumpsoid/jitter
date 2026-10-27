//! The search page: the search controller, the calls it needs, its state
//! ↔ signals, and the pause in typing it waits for.
//!
//! Every keystroke arrives as `SearchTyped`. The text waits in
//! [`SearchPanel`] until typing pauses for [`DEBOUNCE`]; each keystroke
//! moves the deadline. That is transport, like the chat batch: what a
//! query is, and which results are current, core decides.
//!
//! Each list goes out like the top categories: a loaded list that is still
//! loaded goes out as an append of its new results, anything else as a
//! snapshot.

use std::time::Duration;

use jitter_core::models::{
    Category, FirstPage, FoundChannel, ListState, MorePages, Page, PageRequest, SearchKind,
    SearchQuery,
};
use jitter_core::search::SearchController;
use jitter_twitch::Twitch;
use rinf::RustSignal;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::Instant;

use crate::format::{followers_label, twitch_error, viewers_label};
use crate::run::Fetched;
use crate::signals::{
    CategoryTile, ChannelResult, SearchCategoriesState, SearchChannelsState, SearchList,
    SearchMore, SearchUpdate,
};

/// How long typing must pause before the text is searched.
const DEBOUNCE: Duration = Duration::from_millis(300);

/// The search controller and the text typed since the last search.
#[derive(Default)]
pub struct SearchPanel {
    search: SearchController,
    typed: Option<Typed>,
}

/// Text waiting for typing to pause.
struct Typed {
    text: String,
    due: Instant,
}

/// A search call's result, back in the loop as [`Fetched::Search`].
pub enum SearchFetched {
    Channels {
        request: PageRequest,
        result: Result<Page<FoundChannel>, String>,
    },
    Categories {
        request: PageRequest,
        result: Result<Page<Category>, String>,
    },
}

/// `SearchTyped`: holds `text` until typing pauses; a later keystroke
/// replaces it and moves the deadline.
pub fn typed(panel: &mut SearchPanel, text: String, now: Instant) {
    panel.typed = Some(Typed {
        text,
        due: now + DEBOUNCE,
    });
}

/// When the waiting text is due to be searched; `None` while none waits.
pub fn due_at(panel: &SearchPanel) -> Option<Instant> {
    panel.typed.as_ref().map(|t| t.due)
}

/// Typing paused: searches the waiting text, or clears the results when it
/// is not a query. Sends both lists either way, so a page that asks again
/// for the query it shows (after a hot restart) gets it.
pub fn settle(panel: &mut SearchPanel, twitch: &Twitch, fetch: &UnboundedSender<Fetched>) {
    let Some(typed) = panel.typed.take() else {
        return;
    };
    let search = &mut panel.search;
    match SearchQuery::parse(&typed.text) {
        Ok(query) => {
            if let Some(requests) = search.search(query) {
                spawn_channels(search, requests.channels, twitch, fetch);
                spawn_categories(search, requests.categories, twitch, fetch);
            }
        }
        Err(_) => search.clear(),
    }
    channels_since(None, search.channels()).send_signal_to_dart();
    categories_since(None, search.categories()).send_signal_to_dart();
}

/// `RefreshSearch`: starts list `list` over, unless nothing is searched.
pub fn refresh(
    panel: &mut SearchPanel,
    list: SearchList,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
) {
    let search = &mut panel.search;
    let kind = SearchKind::from(list);
    if let Some(request) = search.begin_refresh(kind) {
        spawn(search, kind, request, twitch, fetch);
        send(None, search, kind);
    }
}

/// `NextSearchPage`: fetches list `list`'s next page, unless core says
/// there is none to start now.
pub fn next(
    panel: &mut SearchPanel,
    list: SearchList,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
) {
    let search = &mut panel.search;
    let kind = SearchKind::from(list);
    let shown = shown_of(search, kind);
    if let Some(request) = search.begin_next(kind) {
        spawn(search, kind, request, twitch, fetch);
        send(shown, search, kind);
    }
}

/// A fetched page, handed to core; core ignores one for an earlier query.
pub fn on_fetched(panel: &mut SearchPanel, fetched: SearchFetched) {
    let search = &mut panel.search;
    match fetched {
        SearchFetched::Channels { request, result } => {
            let shown = shown(search.channels());
            search.apply_channels(request, result);
            channels_since(shown, search.channels()).send_signal_to_dart();
        }
        SearchFetched::Categories { request, result } => {
            let shown = shown(search.categories());
            search.apply_categories(request, result);
            categories_since(shown, search.categories()).send_signal_to_dart();
        }
    }
}

fn shown_of(search: &SearchController, kind: SearchKind) -> Option<usize> {
    match kind {
        SearchKind::Channels => shown(search.channels()),
        SearchKind::Categories => shown(search.categories()),
    }
}

fn send(shown: Option<usize>, search: &SearchController, kind: SearchKind) {
    match kind {
        SearchKind::Channels => channels_since(shown, search.channels()),
        SearchKind::Categories => categories_since(shown, search.categories()),
    }
    .send_signal_to_dart();
}

fn spawn(
    search: &SearchController,
    kind: SearchKind,
    request: PageRequest,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
) {
    match kind {
        SearchKind::Channels => spawn_channels(search, request, twitch, fetch),
        SearchKind::Categories => spawn_categories(search, request, twitch, fetch),
    }
}

/// Fetches `request` of the channels matching the current query.
fn spawn_channels(
    search: &SearchController,
    request: PageRequest,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
) {
    let Some(query) = search.query().cloned() else {
        return;
    };
    let (twitch, fetch) = (twitch.clone(), fetch.clone());
    tokio::spawn(async move {
        let result = twitch
            .search_channels(&query, request.cursor.as_ref())
            .await
            .map_err(|e| twitch_error(&e));
        let fetched = SearchFetched::Channels { request, result };
        let _ = fetch.send(Fetched::Search(fetched));
    });
}

/// Fetches `request` of the categories matching the current query.
fn spawn_categories(
    search: &SearchController,
    request: PageRequest,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
) {
    let Some(query) = search.query().cloned() else {
        return;
    };
    let (twitch, fetch) = (twitch.clone(), fetch.clone());
    tokio::spawn(async move {
        let result = twitch
            .search_categories(&query, request.cursor.as_ref())
            .await
            .map_err(|e| twitch_error(&e));
        let fetched = SearchFetched::Categories { request, result };
        let _ = fetch.send(Fetched::Search(fetched));
    });
}

/// How many results Dart lists after the last update, when it lists a
/// loaded page.
fn shown<T>(state: &ListState<T>) -> Option<usize> {
    match state {
        ListState::Loaded { items, .. } => Some(items.len()),
        ListState::Idle | ListState::FirstPage(_) => None,
    }
}

/// The update that takes Dart from `shown` channels to `state`.
fn channels_since(shown: Option<usize>, state: &ListState<FoundChannel>) -> SearchUpdate {
    let state = match (shown, state) {
        (Some(shown), ListState::Loaded { items, more, .. }) => {
            return SearchUpdate::ChannelsAppended {
                channels: rows(items.get(shown..).unwrap_or_default()),
                more: more.into(),
            };
        }
        (_, ListState::Idle) => SearchChannelsState::Idle,
        (_, ListState::FirstPage(FirstPage::Loading)) => SearchChannelsState::Loading,
        (_, ListState::FirstPage(FirstPage::Failed { message })) => SearchChannelsState::Failed {
            message: message.clone(),
        },
        (None, ListState::Loaded { items, more, .. }) => SearchChannelsState::Loaded {
            channels: rows(items),
            more: more.into(),
        },
    };
    SearchUpdate::Channels { state }
}

/// The update that takes Dart from `shown` categories to `state`.
fn categories_since(shown: Option<usize>, state: &ListState<Category>) -> SearchUpdate {
    let state = match (shown, state) {
        (Some(shown), ListState::Loaded { items, more, .. }) => {
            return SearchUpdate::CategoriesAppended {
                categories: tiles(items.get(shown..).unwrap_or_default()),
                more: more.into(),
            };
        }
        (_, ListState::Idle) => SearchCategoriesState::Idle,
        (_, ListState::FirstPage(FirstPage::Loading)) => SearchCategoriesState::Loading,
        (_, ListState::FirstPage(FirstPage::Failed { message })) => SearchCategoriesState::Failed {
            message: message.clone(),
        },
        (None, ListState::Loaded { items, more, .. }) => SearchCategoriesState::Loaded {
            categories: tiles(items),
            more: more.into(),
        },
    };
    SearchUpdate::Categories { state }
}

fn rows(channels: &[FoundChannel]) -> Vec<ChannelResult> {
    channels.iter().map(ChannelResult::from).collect()
}

fn tiles(categories: &[Category]) -> Vec<CategoryTile> {
    categories.iter().map(CategoryTile::from).collect()
}

impl From<SearchList> for SearchKind {
    fn from(list: SearchList) -> Self {
        match list {
            SearchList::Channels => Self::Channels,
            SearchList::Categories => Self::Categories,
        }
    }
}

impl From<&MorePages> for SearchMore {
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

impl From<&FoundChannel> for ChannelResult {
    fn from(found: &FoundChannel) -> Self {
        let channel = &found.channel;
        Self {
            login: channel.login.as_str().to_owned(),
            display_name: channel.display_name.clone(),
            avatar_url: channel.avatar().map(str::to_owned),
            followers_label: found.followers.map(followers_label),
            live_label: found.viewers.map(viewers_label),
        }
    }
}

#[cfg(test)]
mod tests {
    use jitter_core::models::{Channel, ChannelId, ChannelLogin, Cursor};

    use super::*;

    type TestResult = Result<(), String>;

    fn found(id: &str, viewers: Option<u32>) -> Result<FoundChannel, String> {
        Ok(FoundChannel {
            channel: Channel {
                id: ChannelId::parse(id).map_err(|e| format!("{e:?}"))?,
                login: ChannelLogin::parse(&format!("user{id}")).map_err(|e| format!("{e:?}"))?,
                display_name: format!("User{id}"),
                avatar_url: String::new(),
            },
            followers: Some(39_950),
            viewers,
        })
    }

    fn page(ids: &[&str], next: Option<&str>) -> Result<Page<FoundChannel>, String> {
        Ok(Page {
            items: ids
                .iter()
                .map(|id| found(id, None))
                .collect::<Result<_, _>>()?,
            next: next.map(Cursor::new),
        })
    }

    fn query(raw: &str) -> Result<SearchQuery, String> {
        SearchQuery::parse(raw).map_err(|e| format!("{e:?}"))
    }

    #[test]
    fn a_row_is_ready_to_show() -> TestResult {
        let row = ChannelResult::from(&found("12", Some(513))?);
        assert_eq!(row.login, "user12");
        assert_eq!(row.display_name, "User12");
        assert_eq!(row.avatar_url, None);
        assert_eq!(row.followers_label.as_deref(), Some("39.9K followers"));
        assert_eq!(row.live_label.as_deref(), Some("513 viewers"));

        let offline = ChannelResult::from(&found("13", None)?);
        assert_eq!(offline.live_label, None);
        Ok(())
    }

    #[test]
    fn a_next_page_appends_only_its_new_channels() -> TestResult {
        let mut search = SearchController::new();
        let requests = search.search(query("user")?).ok_or("no search")?;
        search.apply_channels(requests.channels, Ok(page(&["1", "2"], Some("c"))?));
        let request = search
            .begin_next(SearchKind::Channels)
            .ok_or("no next page")?;
        let shown = shown(search.channels());
        search.apply_channels(request, Ok(page(&["2", "3"], None)?));
        let SearchUpdate::ChannelsAppended { channels, more } =
            channels_since(shown, search.channels())
        else {
            return Err("not appended".into());
        };
        let logins: Vec<&str> = channels.iter().map(|c| c.login.as_str()).collect();
        assert_eq!(logins, ["user3"]);
        assert!(matches!(more, SearchMore::Idle));
        Ok(())
    }

    #[test]
    fn a_cleared_search_sends_idle_lists() {
        let search = SearchController::new();
        assert!(matches!(
            channels_since(None, search.channels()),
            SearchUpdate::Channels {
                state: SearchChannelsState::Idle
            }
        ));
        assert!(matches!(
            categories_since(None, search.categories()),
            SearchUpdate::Categories {
                state: SearchCategoriesState::Idle
            }
        ));
    }

    #[test]
    fn each_keystroke_moves_the_deadline_and_the_last_text_waits() {
        let mut panel = SearchPanel::default();
        assert_eq!(due_at(&panel), None);

        let start = Instant::now();
        typed(&mut panel, "l".to_owned(), start);
        assert_eq!(due_at(&panel), Some(start + DEBOUNCE));

        let later = start + Duration::from_millis(120);
        typed(&mut panel, "li".to_owned(), later);
        assert_eq!(due_at(&panel), Some(later + DEBOUNCE));
        assert_eq!(panel.typed.as_ref().map(|t| t.text.as_str()), Some("li"));
    }
}
