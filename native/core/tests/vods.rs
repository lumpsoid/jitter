use jitter_core::models::{
    Channel, ChannelId, ChannelLogin, Cursor, FirstPage, ListState, Page, Seconds, Timestamp,
    Video, VideoId, VideoKind,
};
use jitter_core::vods::{FIRST_KIND, VodsController};

fn login(value: &str) -> ChannelLogin {
    let Ok(login) = ChannelLogin::parse(value) else {
        unreachable!("literal is valid");
    };
    login
}

fn video(id: &str, kind: VideoKind) -> Video {
    let (Ok(channel_id), Ok(video_id)) = (ChannelId::parse("1"), VideoId::parse(id)) else {
        unreachable!("literals are valid");
    };
    Video {
        id: video_id,
        channel: Channel {
            id: channel_id,
            login: login("xqc"),
            display_name: "xqc".to_owned(),
            avatar_url: String::new(),
        },
        title: String::new(),
        duration: Seconds::new(60),
        published_at: Timestamp::from_unix(0),
        thumbnail_url: String::new(),
        kind,
    }
}

fn page(ids: &[&str], kind: VideoKind, next: Option<&str>) -> Page<Video> {
    Page {
        items: ids.iter().map(|id| video(id, kind)).collect(),
        next: next.map(Cursor::new),
    }
}

fn ids(c: &VodsController) -> Vec<String> {
    match c.state() {
        ListState::Loaded { items, .. } => items.iter().map(|v| v.id.to_string()).collect(),
        other => unreachable!("not loaded: {other:?}"),
    }
}

#[test]
fn new_controller_is_idle_until_refreshed() {
    let c = VodsController::new(login("xqc"), VideoKind::Archive);
    assert_eq!(c.state(), &ListState::Idle);
    assert_eq!(c.kind(), VideoKind::Archive);
    assert_eq!(c.channel(), &login("xqc"));
}

#[test]
fn switching_kind_mid_flight_ignores_the_old_first_page() {
    let mut c = VodsController::new(login("xqc"), VideoKind::Archive);
    let archives = c.begin_refresh();
    let Some(highlights) = c.show_kind(VideoKind::Highlight) else {
        unreachable!("another kind starts over");
    };
    assert_eq!(c.kind(), VideoKind::Highlight);

    c.apply_page(archives, Ok(page(&["1"], VideoKind::Archive, None)));
    assert_eq!(c.state(), &ListState::FirstPage(FirstPage::Loading));

    c.apply_page(highlights, Ok(page(&["2"], VideoKind::Highlight, None)));
    assert_eq!(ids(&c), ["2"]);
}

#[test]
fn switching_kind_while_next_page_loads_ignores_that_page() {
    let mut c = VodsController::new(login("xqc"), VideoKind::Archive);
    let r = c.begin_refresh();
    c.apply_page(r, Ok(page(&["1"], VideoKind::Archive, Some("c1"))));
    let Some(old_next) = c.begin_next() else {
        unreachable!("page 2 available");
    };
    let Some(uploads) = c.show_kind(VideoKind::Upload) else {
        unreachable!("another kind starts over");
    };
    c.apply_page(uploads, Ok(page(&["5"], VideoKind::Upload, Some("u1"))));
    c.apply_page(old_next, Ok(page(&["2"], VideoKind::Archive, None)));
    assert_eq!(ids(&c), ["5"]);
    assert!(c.begin_next().is_some());
}

#[test]
fn showing_the_kind_already_shown_keeps_its_list() {
    let mut c = VodsController::new(login("xqc"), VideoKind::Archive);
    let r = c.begin_refresh();
    c.apply_page(r, Ok(page(&["1"], VideoKind::Archive, Some("c1"))));
    assert_eq!(c.show_kind(VideoKind::Archive), None);
    assert_eq!(ids(&c), ["1"]);
    assert!(c.begin_next().is_some());
}

#[test]
fn switching_channel_starts_a_new_list() {
    let mut c = VodsController::new(login("xqc"), VideoKind::Archive);
    let old = c.begin_refresh();
    let fresh = c.show_channel(login("shroud"));
    assert_eq!(c.channel(), &login("shroud"));
    c.apply_page(old, Ok(page(&["1"], VideoKind::Archive, None)));
    assert_eq!(c.state(), &ListState::FirstPage(FirstPage::Loading));
    c.apply_page(fresh, Ok(page(&["9"], VideoKind::Archive, None)));
    assert_eq!(ids(&c), ["9"]);
}

#[test]
fn showing_a_channel_and_kind_starts_one_new_list() {
    let mut c = VodsController::new(login("xqc"), VideoKind::Upload);
    let old = c.begin_refresh();
    let fresh = c.show(login("shroud"), FIRST_KIND);
    assert_eq!(c.channel(), &login("shroud"));
    assert_eq!(c.kind(), VideoKind::Archive);
    c.apply_page(old, Ok(page(&["1"], VideoKind::Upload, None)));
    assert_eq!(c.state(), &ListState::FirstPage(FirstPage::Loading));
    c.apply_page(fresh, Ok(page(&["9"], VideoKind::Archive, None)));
    assert_eq!(ids(&c), ["9"]);
}

#[test]
fn duplicate_videos_across_pages_are_dropped() {
    let mut c = VodsController::new(login("xqc"), VideoKind::Archive);
    let r = c.begin_refresh();
    c.apply_page(r, Ok(page(&["1", "2"], VideoKind::Archive, Some("c1"))));
    let Some(r) = c.begin_next() else {
        unreachable!("page 2 available");
    };
    c.apply_page(r, Ok(page(&["2", "3"], VideoKind::Archive, None)));
    assert_eq!(ids(&c), ["1", "2", "3"]);
    assert!(c.begin_next().is_none());
}
