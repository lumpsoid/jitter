use jitter_core::chat_replay::{BACKFILL, ChatReplay, LOW_WATER, ReplayRequest, SEEK_JUMP};
use jitter_core::models::{
    ChannelId, ChannelLogin, ChatChange, ChatEntry, ChatMessage, Chatter, Cursor, Page,
    ReplayComment, ReplayFrom, Seconds, Timestamp, VideoId,
};
use jitter_core::playback::PlaybackTarget;

/// Any loaded player: only zero means "not loaded".
const DURATION: Seconds = Seconds::new(10_000);

fn video(raw: &str) -> VideoId {
    let Ok(id) = VideoId::parse(raw) else {
        unreachable!("valid id");
    };
    id
}

fn vod() -> PlaybackTarget {
    PlaybackTarget::Vod(video("1"))
}

fn s(value: u32) -> Seconds {
    Seconds::new(value)
}

fn comment(id: &str, offset: u32) -> ReplayComment {
    let Ok(user_id) = ChannelId::parse("7") else {
        unreachable!("valid id");
    };
    ReplayComment {
        offset: s(offset),
        message: ChatMessage {
            id: id.to_owned(),
            sender: Chatter {
                user_id,
                display_name: "n".to_owned(),
                color: None,
                badges: vec![],
            },
            text: "hi".to_owned(),
            action: false,
            emotes: vec![],
            sent_at: Timestamp::from_unix(0),
            deleted: false,
        },
    }
}

fn page(comments: &[(&str, u32)], next: Option<&str>) -> Page<ReplayComment> {
    Page {
        items: comments.iter().map(|(id, at)| comment(id, *at)).collect(),
        next: next.map(Cursor::new),
    }
}

fn at(offset: u32) -> ReplayRequest {
    ReplayRequest {
        video: video("1"),
        from: ReplayFrom::Offset(s(offset)),
    }
}

fn after(cursor: &str) -> ReplayRequest {
    ReplayRequest {
        video: video("1"),
        from: ReplayFrom::After(Cursor::new(cursor)),
    }
}

fn shown(replay: &ChatReplay) -> Vec<String> {
    replay
        .chat()
        .entries()
        .filter_map(ChatEntry::message)
        .map(|m| m.id.clone())
        .collect()
}

fn appended(changes: &[ChatChange]) -> Vec<String> {
    changes
        .iter()
        .filter_map(|c| match c {
            ChatChange::Appended { entry } => entry.message().map(|m| m.id.clone()),
            _ => None,
        })
        .collect()
}

/// A replay at `position` with the first page applied: `a` at 95, `b` at
/// 100, `c` at 105, `d` at 110, and more after cursor `p2`. Pads the page
/// past `LOW_WATER` so it asks for nothing more.
fn started(position: u32) -> ChatReplay {
    let mut replay = ChatReplay::new(video("1"));
    let step = replay.position(&vod(), s(position), DURATION);
    let Some(request) = step.fetch else {
        unreachable!("the first position fetches");
    };
    let mut items = vec![("a", 95), ("b", 100), ("c", 105), ("d", 110)];
    let pad: Vec<String> = (0..=LOW_WATER).map(|i| format!("z{i}")).collect();
    items.extend(pad.iter().map(|id| (id.as_str(), 1000)));
    let step = replay.apply(request, Ok(page(&items, Some("p2"))));
    assert_eq!(step.fetch, None);
    replay
}

#[test]
fn nothing_is_fetched_until_the_player_has_a_position() {
    let mut replay = ChatReplay::new(video("1"));
    assert!(replay.waiting());
    let step = replay.position(&vod(), s(100), Seconds::ZERO);
    assert_eq!(step.fetch, None);
    assert!(replay.waiting());
}

#[test]
fn the_first_position_loads_the_chat_from_a_little_before_it() {
    let mut replay = ChatReplay::new(video("1"));
    let step = replay.position(&vod(), s(100), DURATION);
    assert_eq!(step.fetch, Some(at(100 - BACKFILL.get())));
    assert!(step.changes.is_empty());
    assert!(replay.waiting());
}

#[test]
fn the_backfill_stops_at_the_start_of_the_video() {
    let mut replay = ChatReplay::new(video("1"));
    let step = replay.position(&vod(), s(10), DURATION);
    assert_eq!(step.fetch, Some(at(0)));
}

#[test]
fn a_page_shows_what_was_said_up_to_the_position() {
    let mut replay = ChatReplay::new(video("1"));
    replay.position(&vod(), s(100), DURATION);
    let step = replay.apply(at(70), Ok(page(&[("a", 95), ("b", 100), ("c", 105)], None)));
    assert_eq!(appended(&step.changes), ["a", "b"]);
    // Waiting ended: the head redraws the status.
    assert!(step.changes.contains(&ChatChange::ConnectionChanged));
    assert!(!replay.waiting());
    assert_eq!(shown(&replay), ["a", "b"]);
}

#[test]
fn playback_shows_each_message_when_the_video_reaches_it() {
    let mut replay = started(100);
    assert_eq!(shown(&replay), ["a", "b"]);
    let step = replay.position(&vod(), s(104), DURATION);
    assert!(step.changes.is_empty());
    let step = replay.position(&vod(), s(105), DURATION);
    assert_eq!(appended(&step.changes), ["c"]);
    let step = replay.position(&vod(), s(112), DURATION);
    assert_eq!(appended(&step.changes), ["d"]);
    assert_eq!(step.fetch, None);
}

#[test]
fn the_next_page_is_asked_for_when_few_messages_are_left() {
    let mut replay = ChatReplay::new(video("1"));
    replay.position(&vod(), s(100), DURATION);
    let step = replay.apply(
        at(70),
        Ok(page(&[("a", 95), ("b", 101), ("c", 102)], Some("p2"))),
    );
    // Few left after showing `a`: the next page is asked for at once.
    assert_eq!(step.fetch, Some(after("p2")));
    // Only once while it is in flight.
    let step = replay.position(&vod(), s(101), DURATION);
    assert_eq!(appended(&step.changes), ["b"]);
    assert_eq!(step.fetch, None);
    // It carries on from where the last page ended.
    let step = replay.apply(after("p2"), Ok(page(&[("d", 102), ("e", 140)], None)));
    assert_eq!(appended(&step.changes), Vec::<String>::new());
    let step = replay.position(&vod(), s(102), DURATION);
    assert_eq!(appended(&step.changes), ["c", "d"]);
    // The last page: nothing more to ask for.
    assert_eq!(step.fetch, None);
}

#[test]
fn a_page_ending_before_the_position_asks_for_the_next_at_once() {
    let mut replay = ChatReplay::new(video("1"));
    replay.position(&vod(), s(100), DURATION);
    let step = replay.apply(at(70), Ok(page(&[("a", 75), ("b", 80)], Some("p2"))));
    assert_eq!(appended(&step.changes), ["a", "b"]);
    assert_eq!(step.fetch, Some(after("p2")));
    let step = replay.apply(after("p2"), Ok(page(&[("c", 99), ("d", 101)], None)));
    assert_eq!(appended(&step.changes), ["c"]);
}

#[test]
fn a_jump_forward_is_a_seek_that_starts_the_chat_over() {
    let mut replay = started(100);
    let target = 100 + SEEK_JUMP.get() + 1;
    let step = replay.position(&vod(), s(target), DURATION);
    assert_eq!(step.changes[0], ChatChange::Cleared);
    assert_eq!(step.fetch, Some(at(target - BACKFILL.get())));
    assert!(shown(&replay).is_empty());
    assert!(replay.waiting());
    // Messages fetched before the seek are gone with it.
    let step = replay.position(&vod(), s(target + 1), DURATION);
    assert!(appended(&step.changes).is_empty());
}

#[test]
fn a_short_skip_forward_shows_what_was_said_in_between() {
    let mut replay = started(100);
    let step = replay.position(&vod(), s(100 + SEEK_JUMP.get()), DURATION);
    assert_eq!(appended(&step.changes), ["c", "d"]);
    assert_eq!(step.fetch, None);
}

#[test]
fn a_step_back_is_a_seek_that_starts_the_chat_over() {
    let mut replay = started(100);
    let step = replay.position(&vod(), s(99), DURATION);
    assert_eq!(step.changes[0], ChatChange::Cleared);
    assert_eq!(step.fetch, Some(at(99 - BACKFILL.get())));
    let step = replay.apply(at(69), Ok(page(&[("a", 95), ("b", 100)], None)));
    assert_eq!(appended(&step.changes), ["a"]);
    assert_eq!(shown(&replay), ["a"]);
}

#[test]
fn a_page_from_before_a_seek_is_dropped() {
    let mut replay = ChatReplay::new(video("1"));
    replay.position(&vod(), s(100), DURATION);
    replay.position(&vod(), s(500), DURATION);
    let step = replay.apply(at(70), Ok(page(&[("old", 95)], None)));
    assert_eq!(step, Default::default());
    assert!(shown(&replay).is_empty());
    let step = replay.apply(at(470), Ok(page(&[("new", 490)], None)));
    assert_eq!(appended(&step.changes), ["new"]);
}

#[test]
fn a_failed_page_stops_the_chat_until_a_seek() {
    let mut replay = ChatReplay::new(video("1"));
    replay.position(&vod(), s(100), DURATION);
    let step = replay.apply(at(70), Err("offline".to_owned()));
    assert_eq!(step.changes, [ChatChange::ConnectionChanged]);
    assert_eq!(replay.failure(), Some("offline"));
    assert!(!replay.waiting());
    let step = replay.position(&vod(), s(101), DURATION);
    assert_eq!(step.fetch, None);
    // A seek tries again.
    let step = replay.position(&vod(), s(50), DURATION);
    assert_eq!(step.fetch, Some(at(20)));
    assert_eq!(replay.failure(), None);
    assert!(step.changes.contains(&ChatChange::ConnectionChanged));
}

#[test]
fn reports_from_another_player_are_ignored() {
    let mut replay = ChatReplay::new(video("1"));
    let other = PlaybackTarget::Vod(video("2"));
    assert_eq!(
        replay.position(&other, s(100), DURATION),
        Default::default()
    );
    let Ok(login) = ChannelLogin::parse("xqc") else {
        unreachable!("valid login");
    };
    assert_eq!(
        replay.position(&PlaybackTarget::Live(login), s(100), DURATION),
        Default::default()
    );
    assert!(replay.waiting());
}

/// Hiding the panel drops the replay; showing it starts a fresh one,
/// which picks up wherever the player is when it next reports.
#[test]
fn a_fresh_replay_resumes_at_the_players_position() {
    let mut replay = started(100);
    replay.position(&vod(), s(110), DURATION);
    drop(replay);
    let mut shown_again = ChatReplay::new(video("1"));
    let step = shown_again.position(&vod(), s(400), DURATION);
    assert_eq!(step.fetch, Some(at(400 - BACKFILL.get())));
    assert!(step.changes.is_empty());
}
