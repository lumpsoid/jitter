use jitter_core::browse::{BrowseController, FirstPage, ListState, MorePages};
use jitter_core::models::{Channel, ChannelId, ChannelLogin, Cursor, Page, Stream, Timestamp};

fn stream(id: &str) -> Stream {
    let (Ok(channel_id), Ok(login)) = (ChannelId::parse("1"), ChannelLogin::parse("xqc")) else {
        unreachable!("literals are valid");
    };
    Stream {
        id: id.to_owned(),
        channel: Channel {
            id: channel_id,
            login,
            display_name: "xqc".to_owned(),
            avatar_url: String::new(),
        },
        title: String::new(),
        game_name: None,
        viewers: 1,
        started_at: Timestamp::from_unix(0),
        thumbnail_url: String::new(),
    }
}

fn page(ids: &[&str], next: Option<&str>) -> Page<Stream> {
    Page {
        items: ids.iter().map(|id| stream(id)).collect(),
        next: next.map(Cursor::new),
    }
}

fn ids(c: &BrowseController) -> Vec<String> {
    match c.state() {
        ListState::Loaded { items, .. } => items.iter().map(|s| s.id.clone()).collect(),
        other => unreachable!("not loaded: {other:?}"),
    }
}

/// A controller holding pages 1 and 2, with a cursor for page 3.
fn loaded_two_pages() -> BrowseController {
    let mut c = BrowseController::new();
    let r = c.begin_refresh();
    c.apply_page(r, Ok(page(&["a", "b"], Some("c1"))));
    let Some(r) = c.begin_next() else {
        unreachable!("page 2 available");
    };
    c.apply_page(r, Ok(page(&["c", "d"], Some("c2"))));
    c
}

#[test]
fn second_begin_next_while_in_flight_is_none() {
    let mut c = BrowseController::new();
    let r = c.begin_refresh();
    c.apply_page(r, Ok(page(&["a"], Some("c1"))));
    assert!(c.begin_next().is_some());
    assert!(c.begin_next().is_none());
}

#[test]
fn begin_next_before_first_page_is_none() {
    let mut c = BrowseController::new();
    assert!(c.begin_next().is_none());
    c.begin_refresh();
    assert!(c.begin_next().is_none());
}

#[test]
fn duplicate_ids_across_and_within_pages_are_dropped() {
    let mut c = BrowseController::new();
    let r = c.begin_refresh();
    c.apply_page(r, Ok(page(&["a", "b", "a"], Some("c1"))));
    let Some(r) = c.begin_next() else {
        unreachable!("next available");
    };
    c.apply_page(r, Ok(page(&["b", "c"], None)));
    assert_eq!(ids(&c), ["a", "b", "c"]);
}

#[test]
fn no_next_cursor_stops_paging() {
    let mut c = BrowseController::new();
    let r = c.begin_refresh();
    c.apply_page(r, Ok(page(&["a"], None)));
    assert!(c.begin_next().is_none());
}

#[test]
fn failed_page_three_keeps_earlier_pages_and_retry_succeeds() {
    let mut c = loaded_two_pages();
    let Some(r) = c.begin_next() else {
        unreachable!("page 3 available");
    };
    c.apply_page(r, Err("boom".to_owned()));
    assert_eq!(ids(&c), ["a", "b", "c", "d"]);
    assert!(matches!(
        c.state(),
        ListState::Loaded { more: MorePages::Failed { message }, .. } if message == "boom"
    ));

    let Some(r) = c.begin_next() else {
        unreachable!("retry allowed");
    };
    assert_eq!(r.cursor, Some(Cursor::new("c2")));
    c.apply_page(r, Ok(page(&["e"], None)));
    assert_eq!(ids(&c), ["a", "b", "c", "d", "e"]);
    assert!(matches!(
        c.state(),
        ListState::Loaded {
            more: MorePages::Idle,
            next: None,
            ..
        }
    ));
}

#[test]
fn page_from_before_a_refresh_is_ignored() {
    let mut c = loaded_two_pages();
    let Some(old) = c.begin_next() else {
        unreachable!("page 3 available");
    };
    let fresh = c.begin_refresh();
    c.apply_page(old, Ok(page(&["stale"], None)));
    assert_eq!(c.state(), &ListState::FirstPage(FirstPage::Loading));
    c.apply_page(fresh, Ok(page(&["x"], None)));
    assert_eq!(ids(&c), ["x"]);
}

#[test]
fn failed_first_page_is_first_page_failed_and_refresh_retries() {
    let mut c = BrowseController::new();
    let r = c.begin_refresh();
    c.apply_page(r, Err("down".to_owned()));
    assert_eq!(
        c.state(),
        &ListState::FirstPage(FirstPage::Failed {
            message: "down".to_owned()
        })
    );
    let r = c.begin_refresh();
    c.apply_page(r, Ok(page(&["a"], None)));
    assert_eq!(ids(&c), ["a"]);
}
