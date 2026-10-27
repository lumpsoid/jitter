use jitter_core::models::{
    Category, CategoryId, Channel, ChannelId, ChannelLogin, Cursor, FirstPage, FoundChannel,
    ListState, MorePages, Page, SearchKind, SearchQuery, SearchQueryError,
};
use jitter_core::search::SearchController;

fn query(raw: &str) -> SearchQuery {
    SearchQuery::parse(raw).unwrap_or_else(|e| unreachable!("{e:?}"))
}

fn found(id: &str) -> FoundChannel {
    FoundChannel {
        channel: Channel {
            id: ChannelId::parse(id).unwrap_or_else(|e| unreachable!("{e:?}")),
            login: ChannelLogin::parse(&format!("user{id}"))
                .unwrap_or_else(|e| unreachable!("{e:?}")),
            display_name: format!("User{id}"),
            avatar_url: String::new(),
        },
        followers: Some(10),
        viewers: None,
    }
}

fn category(id: &str) -> Category {
    Category {
        id: CategoryId::parse(id).unwrap_or_else(|e| unreachable!("{e:?}")),
        name: format!("Game {id}"),
        box_art_url: String::new(),
        viewers: 0,
    }
}

fn channels(ids: &[&str], next: Option<&str>) -> Page<FoundChannel> {
    Page {
        items: ids.iter().map(|id| found(id)).collect(),
        next: next.map(Cursor::new),
    }
}

fn categories(ids: &[&str], next: Option<&str>) -> Page<Category> {
    Page {
        items: ids.iter().map(|id| category(id)).collect(),
        next: next.map(Cursor::new),
    }
}

fn channel_ids(state: &ListState<FoundChannel>) -> Vec<&str> {
    match state {
        ListState::Loaded { items, .. } => items.iter().map(|c| c.channel.id.as_str()).collect(),
        ListState::Idle | ListState::FirstPage(_) => Vec::new(),
    }
}

fn category_ids(state: &ListState<Category>) -> Vec<&str> {
    match state {
        ListState::Loaded { items, .. } => items.iter().map(|c| c.id.as_str()).collect(),
        ListState::Idle | ListState::FirstPage(_) => Vec::new(),
    }
}

// --- the query --------------------------------------------------------------

#[test]
fn a_query_is_the_text_without_surrounding_whitespace() {
    assert_eq!(query("  elden ring \n").as_str(), "elden ring");
    assert_eq!(query("xqc").as_str(), "xqc");
}

#[test]
fn blank_text_is_not_a_query() {
    assert_eq!(SearchQuery::parse(""), Err(SearchQueryError::Empty));
    assert_eq!(SearchQuery::parse("  \t\n"), Err(SearchQueryError::Empty));
}

// --- the controller ---------------------------------------------------------

#[test]
fn nothing_is_loaded_before_a_search() {
    let search = SearchController::new();
    assert_eq!(search.query(), None);
    assert_eq!(*search.channels(), ListState::Idle);
    assert_eq!(*search.categories(), ListState::Idle);
}

#[test]
fn a_search_loads_both_lists_from_their_first_page() {
    let mut search = SearchController::new();
    let requests = search
        .search(query("lirik"))
        .unwrap_or_else(|| unreachable!());
    assert_eq!(requests.channels.cursor, None);
    assert_eq!(requests.categories.cursor, None);
    assert_eq!(*search.channels(), ListState::FirstPage(FirstPage::Loading));
    assert_eq!(
        *search.categories(),
        ListState::FirstPage(FirstPage::Loading)
    );

    search.apply_channels(requests.channels, Ok(channels(&["1", "2"], Some("c"))));
    search.apply_categories(requests.categories, Ok(categories(&["9"], None)));

    assert_eq!(search.query().map(SearchQuery::as_str), Some("lirik"));
    assert_eq!(channel_ids(search.channels()), ["1", "2"]);
    assert_eq!(category_ids(search.categories()), ["9"]);
}

#[test]
fn each_list_pages_on_its_own() {
    let mut search = SearchController::new();
    let requests = search
        .search(query("lirik"))
        .unwrap_or_else(|| unreachable!());
    search.apply_channels(requests.channels, Ok(channels(&["1"], Some("c1"))));
    search.apply_categories(requests.categories, Ok(categories(&["9"], None)));

    assert_eq!(search.begin_next(SearchKind::Categories), None);
    let next = search
        .begin_next(SearchKind::Channels)
        .unwrap_or_else(|| unreachable!());
    assert_eq!(next.cursor, Some(Cursor::new("c1")));
    assert_eq!(search.begin_next(SearchKind::Channels), None);

    search.apply_channels(next, Ok(channels(&["1", "2"], None)));
    assert_eq!(channel_ids(search.channels()), ["1", "2"]);
    assert_eq!(search.begin_next(SearchKind::Channels), None);
}

#[test]
fn a_failed_next_page_keeps_the_results_and_can_be_retried() {
    let mut search = SearchController::new();
    let requests = search
        .search(query("lirik"))
        .unwrap_or_else(|| unreachable!());
    search.apply_channels(requests.channels, Ok(channels(&["1"], Some("c1"))));
    let next = search
        .begin_next(SearchKind::Channels)
        .unwrap_or_else(|| unreachable!());
    search.apply_channels(next, Err("timed out".into()));

    let ListState::Loaded { items, more, .. } = search.channels() else {
        unreachable!("not loaded")
    };
    assert_eq!(items.len(), 1);
    assert_eq!(
        *more,
        MorePages::Failed {
            message: "timed out".into()
        }
    );
    assert!(search.begin_next(SearchKind::Channels).is_some());
}

#[test]
fn results_for_a_query_no_longer_current_are_dropped() {
    let mut search = SearchController::new();
    let old = search
        .search(query("lir"))
        .unwrap_or_else(|| unreachable!());
    let new = search
        .search(query("lirik"))
        .unwrap_or_else(|| unreachable!());

    search.apply_channels(old.channels, Ok(channels(&["1"], None)));
    search.apply_categories(old.categories, Ok(categories(&["9"], None)));
    assert_eq!(*search.channels(), ListState::FirstPage(FirstPage::Loading));
    assert_eq!(
        *search.categories(),
        ListState::FirstPage(FirstPage::Loading)
    );

    search.apply_channels(new.channels, Ok(channels(&["2"], None)));
    search.apply_categories(new.categories, Ok(categories(&["8"], None)));
    assert_eq!(channel_ids(search.channels()), ["2"]);
    assert_eq!(category_ids(search.categories()), ["8"]);
}

#[test]
fn a_next_page_for_the_query_before_is_dropped() {
    let mut search = SearchController::new();
    let requests = search
        .search(query("lir"))
        .unwrap_or_else(|| unreachable!());
    search.apply_channels(requests.channels, Ok(channels(&["1"], Some("c1"))));
    let stale = search
        .begin_next(SearchKind::Channels)
        .unwrap_or_else(|| unreachable!());
    let new = search
        .search(query("lirik"))
        .unwrap_or_else(|| unreachable!());
    search.apply_channels(new.channels, Ok(channels(&["2"], None)));

    search.apply_channels(stale, Ok(channels(&["3"], None)));
    assert_eq!(channel_ids(search.channels()), ["2"]);
}

#[test]
fn the_same_query_again_keeps_its_results() {
    let mut search = SearchController::new();
    let requests = search
        .search(query("lirik"))
        .unwrap_or_else(|| unreachable!());
    search.apply_channels(requests.channels, Ok(channels(&["1"], None)));

    assert_eq!(search.search(query(" lirik ")), None);
    assert_eq!(channel_ids(search.channels()), ["1"]);
}

#[test]
fn clearing_forgets_the_query_and_drops_pages_in_flight() {
    let mut search = SearchController::new();
    let requests = search
        .search(query("lirik"))
        .unwrap_or_else(|| unreachable!());
    search.clear();
    search.apply_channels(requests.channels, Ok(channels(&["1"], None)));
    search.apply_categories(requests.categories, Ok(categories(&["9"], None)));

    assert_eq!(search.query(), None);
    assert_eq!(*search.channels(), ListState::Idle);
    assert_eq!(*search.categories(), ListState::Idle);
    assert_eq!(search.begin_refresh(SearchKind::Channels), None);
}

#[test]
fn after_clearing_the_same_query_searches_again() {
    let mut search = SearchController::new();
    let _ = search.search(query("lirik"));
    search.clear();
    assert!(search.search(query("lirik")).is_some());
}

#[test]
fn refreshing_one_list_starts_only_it_over() {
    let mut search = SearchController::new();
    let requests = search
        .search(query("lirik"))
        .unwrap_or_else(|| unreachable!());
    search.apply_channels(requests.channels, Err("offline".into()));
    search.apply_categories(requests.categories, Ok(categories(&["9"], None)));

    let retry = search
        .begin_refresh(SearchKind::Channels)
        .unwrap_or_else(|| unreachable!());
    assert_eq!(*search.channels(), ListState::FirstPage(FirstPage::Loading));
    assert_eq!(category_ids(search.categories()), ["9"]);

    search.apply_channels(retry, Ok(channels(&["1"], None)));
    assert_eq!(channel_ids(search.channels()), ["1"]);
}
