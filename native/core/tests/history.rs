use jitter_core::fakes::FakeHistoryStore;
use jitter_core::history::{HistoryChange, HistoryController};
use jitter_core::history_policy::{MERGE_GAP, MIN_WATCH, TOUCH_STEP};
use jitter_core::models::{
    ChannelLogin, LiveDetails, Playback, PlaybackDetails, QualityKind, QualityPreference,
    RewindOutcome, Seconds, Timestamp, Variant, VideoId, Visit, VisitDetails, VisitId,
};
use jitter_core::playback::{PlaybackController, PlaybackTarget};

fn live(login: &str) -> PlaybackTarget {
    let Ok(l) = ChannelLogin::parse(login) else {
        unreachable!("valid login");
    };
    PlaybackTarget::Live(l)
}

fn vod(id: &str) -> PlaybackTarget {
    let Ok(id) = VideoId::parse(id) else {
        unreachable!("valid id");
    };
    PlaybackTarget::Vod(id)
}

fn at(seconds: i64) -> Timestamp {
    Timestamp::from_unix(seconds)
}

fn secs(seconds: Seconds) -> i64 {
    i64::from(seconds.get())
}

fn titled(title: &str) -> PlaybackDetails {
    PlaybackDetails {
        title: Some(title.to_owned()),
        ..PlaybackDetails::default()
    }
}

/// A player with `target` ready and in front, showing `details`.
fn playing(target: &PlaybackTarget, details: PlaybackDetails) -> PlaybackController {
    let mut playback = PlaybackController::new();
    open(&mut playback, target, details);
    playback
}

fn open(playback: &mut PlaybackController, target: &PlaybackTarget, details: PlaybackDetails) {
    playback.begin(target.clone());
    let quality = Variant {
        name: "720p".to_owned(),
        url: "https://example.test/720p.m3u8".to_owned(),
        resolution: None,
        frame_rate: None,
        bandwidth: 1000,
        kind: QualityKind::Source,
    };
    playback.apply(
        target.clone(),
        Ok(Playback::from_variants(
            "https://example.test/master.m3u8".to_owned(),
            vec![quality],
        )),
        Ok(details),
        QualityPreference::Source,
    );
}

fn controller(store: &FakeHistoryStore) -> HistoryController {
    HistoryController::new(Box::new(store.clone()))
}

/// Reports `target` playing every second from `from` to `to`, both in.
fn play_through(
    history: &mut HistoryController,
    playback: &PlaybackController,
    target: &PlaybackTarget,
    from: i64,
    to: i64,
) -> Result<(), String> {
    for now in from..=to {
        history
            .heard(playback, target, at(now))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn targets(store: &FakeHistoryStore) -> Vec<PlaybackTarget> {
    store.visits().into_iter().map(|v| v.target).collect()
}

#[test]
fn playing_shorter_than_the_minimum_is_not_a_visit() -> Result<(), String> {
    let store = FakeHistoryStore::default();
    let mut history = controller(&store);
    let a = live("a");
    let playback = playing(&a, titled("hello"));
    play_through(&mut history, &playback, &a, 0, secs(MIN_WATCH) - 1)?;
    assert!(store.visits().is_empty());
    assert!(history.visits().is_empty());
    Ok(())
}

#[test]
fn playing_long_enough_stores_a_visit_with_what_was_on() -> Result<(), String> {
    let store = FakeHistoryStore::default();
    let mut history = controller(&store);
    let a = live("a");
    let playback = playing(&a, titled("hello"));
    play_through(&mut history, &playback, &a, 100, 100 + secs(MIN_WATCH) - 1)?;
    assert_eq!(
        history.heard(&playback, &a, at(100 + secs(MIN_WATCH))),
        Ok(HistoryChange::Listed)
    );
    let visits = store.visits();
    let [visit] = visits.as_slice() else {
        return Err(format!("expected one visit, got {visits:?}"));
    };
    assert_eq!(visit.target, a);
    assert_eq!(visit.started_at, at(100));
    assert_eq!(visit.last_seen_at, at(100 + secs(MIN_WATCH)));
    assert_eq!(visit.details.title.as_deref(), Some("hello"));
    assert_eq!(history.visits(), visits.as_slice());
    Ok(())
}

#[test]
fn reports_for_a_target_not_playing_count_for_nothing() -> Result<(), String> {
    let store = FakeHistoryStore::default();
    let mut history = controller(&store);
    let playback = playing(&live("b"), PlaybackDetails::default());
    play_through(&mut history, &playback, &live("a"), 0, 60)?;
    let resolving = {
        let mut p = PlaybackController::new();
        p.begin(live("a"));
        p
    };
    play_through(&mut history, &resolving, &live("a"), 0, 60)?;
    assert!(store.visits().is_empty());
    Ok(())
}

#[test]
fn a_player_stopped_away_is_not_watching() -> Result<(), String> {
    let store = FakeHistoryStore::default();
    let mut history = controller(&store);
    let a = live("a");
    let mut playback = playing(&a, PlaybackDetails::default());
    playback.background(false, true, Seconds::ZERO);
    play_through(&mut history, &playback, &a, 0, 60)?;
    assert!(store.visits().is_empty());
    Ok(())
}

#[test]
fn listening_away_is_watching() -> Result<(), String> {
    let store = FakeHistoryStore::default();
    let mut history = controller(&store);
    let a = live("a");
    let mut playback = playing(&a, PlaybackDetails::default());
    playback.background(true, true, Seconds::ZERO);
    play_through(&mut history, &playback, &a, 0, 60)?;
    assert_eq!(targets(&store), [a]);
    Ok(())
}

#[test]
fn leaving_and_coming_back_soon_carries_the_visit_on() -> Result<(), String> {
    let store = FakeHistoryStore::default();
    let mut history = controller(&store);
    let a = live("a");
    let mut playback = playing(&a, titled("first"));
    play_through(&mut history, &playback, &a, 0, 100)?;
    history
        .leave(&playback, &a, at(100))
        .map_err(|e| e.to_string())?;
    playback.leave(&a);

    let back = 100 + secs(MERGE_GAP);
    open(&mut playback, &a, titled("second"));
    play_through(&mut history, &playback, &a, back, back + 20)?;
    let visits = store.visits();
    let [visit] = visits.as_slice() else {
        return Err(format!("expected one visit, got {visits:?}"));
    };
    assert_eq!(visit.started_at, at(0));
    assert_eq!(visit.last_seen_at, at(back + secs(MIN_WATCH)));
    // What was on when it started.
    assert_eq!(visit.details.title.as_deref(), Some("first"));
    Ok(())
}

#[test]
fn coming_back_after_the_gap_is_a_new_visit_and_the_old_one_stays() -> Result<(), String> {
    let store = FakeHistoryStore::default();
    let mut history = controller(&store);
    let a = live("a");
    let mut playback = playing(&a, PlaybackDetails::default());
    play_through(&mut history, &playback, &a, 0, 100)?;
    history
        .leave(&playback, &a, at(100))
        .map_err(|e| e.to_string())?;
    let first = store.visits();

    let back = 101 + secs(MERGE_GAP);
    open(&mut playback, &a, PlaybackDetails::default());
    play_through(&mut history, &playback, &a, back, back + 20)?;
    let visits = store.visits();
    assert_eq!(visits.len(), 2);
    assert_eq!(visits.first().map(|v| v.started_at), Some(at(back)));
    assert_eq!(visits.get(1), first.first());
    Ok(())
}

#[test]
fn going_back_after_watching_something_else_is_a_new_visit() -> Result<(), String> {
    let store = FakeHistoryStore::default();
    let mut history = controller(&store);
    let (a, b) = (live("a"), vod("42"));
    let mut playback = playing(&a, PlaybackDetails::default());
    play_through(&mut history, &playback, &a, 0, 60)?;
    open(&mut playback, &b, PlaybackDetails::default());
    play_through(&mut history, &playback, &b, 61, 90)?;
    open(&mut playback, &a, PlaybackDetails::default());
    play_through(&mut history, &playback, &a, 91, 120)?;
    assert_eq!(targets(&store), [a.clone(), b, a]);
    Ok(())
}

#[test]
fn a_peek_too_short_to_count_does_not_split_a_visit() -> Result<(), String> {
    let store = FakeHistoryStore::default();
    let mut history = controller(&store);
    let (a, b) = (live("a"), live("b"));
    let mut playback = playing(&a, PlaybackDetails::default());
    play_through(&mut history, &playback, &a, 0, 60)?;
    open(&mut playback, &b, PlaybackDetails::default());
    play_through(&mut history, &playback, &b, 61, 61 + secs(MIN_WATCH) - 2)?;
    open(&mut playback, &a, PlaybackDetails::default());
    play_through(&mut history, &playback, &a, 80, 120)?;
    let visits = store.visits();
    assert_eq!(targets(&store), [a]);
    assert_eq!(visits.first().map(|v| v.last_seen_at), Some(at(120)));
    Ok(())
}

#[test]
fn playing_an_old_visit_again_adds_one_today_and_leaves_that_day_alone() -> Result<(), String> {
    let a = vod("42");
    let tuesday = Visit {
        id: VisitId::from_raw(7),
        target: a.clone(),
        started_at: at(1_000),
        last_seen_at: at(2_000),
        details: VisitDetails::default(),
    };
    let store = FakeHistoryStore::with_visits(vec![tuesday.clone()]);
    let mut history = controller(&store);
    history.load().map_err(|e| e.to_string())?;
    let today = 1_000_000;
    let playback = playing(&a, PlaybackDetails::default());
    play_through(&mut history, &playback, &a, today, today + 30)?;
    let visits = store.visits();
    assert_eq!(visits.len(), 2);
    assert_eq!(visits.first().map(|v| v.started_at), Some(at(today)));
    assert_eq!(visits.get(1), Some(&tuesday));
    Ok(())
}

#[test]
fn a_visit_is_written_every_touch_step_and_once_more_on_leaving() -> Result<(), String> {
    let store = FakeHistoryStore::default();
    let mut history = controller(&store);
    let a = live("a");
    let playback = playing(&a, PlaybackDetails::default());
    let counted = secs(MIN_WATCH);
    play_through(&mut history, &playback, &a, 0, counted)?;
    let last_seen = |store: &FakeHistoryStore| store.visits().first().map(|v| v.last_seen_at);

    let step = secs(TOUCH_STEP);
    play_through(&mut history, &playback, &a, counted + 1, counted + step - 1)?;
    assert_eq!(last_seen(&store), Some(at(counted)));
    assert_eq!(
        history.heard(&playback, &a, at(counted + step)),
        Ok(HistoryChange::Touched)
    );
    assert_eq!(last_seen(&store), Some(at(counted + step)));
    assert_eq!(
        history.visits().first().map(|v| v.last_seen_at),
        Some(at(counted + step))
    );

    play_through(
        &mut history,
        &playback,
        &a,
        counted + step + 1,
        counted + step + 5,
    )?;
    assert_eq!(
        history.leave(&playback, &a, at(counted + step + 5)),
        Ok(HistoryChange::Touched)
    );
    assert_eq!(last_seen(&store), Some(at(counted + step + 5)));
    Ok(())
}

#[test]
fn leaving_before_it_counts_stores_nothing() -> Result<(), String> {
    let store = FakeHistoryStore::default();
    let mut history = controller(&store);
    let a = live("a");
    let playback = playing(&a, PlaybackDetails::default());
    play_through(&mut history, &playback, &a, 0, 5)?;
    assert_eq!(
        history.leave(&playback, &a, at(5)),
        Ok(HistoryChange::Unchanged)
    );
    play_through(&mut history, &playback, &a, 6, 6 + secs(MIN_WATCH) - 1)?;
    assert!(store.visits().is_empty());
    Ok(())
}

#[test]
fn a_long_break_in_one_page_ends_the_visit() -> Result<(), String> {
    let store = FakeHistoryStore::default();
    let mut history = controller(&store);
    let a = live("a");
    let playback = playing(&a, PlaybackDetails::default());
    play_through(&mut history, &playback, &a, 0, 60)?;
    // Slept, say, and resumed the next morning.
    let morning = 61 + secs(MERGE_GAP) + 3_600;
    play_through(&mut history, &playback, &a, morning, morning + 20)?;
    let visits = store.visits();
    assert_eq!(visits.len(), 2);
    assert_eq!(visits.get(1).map(|v| v.last_seen_at), Some(at(60)));
    assert_eq!(visits.first().map(|v| v.started_at), Some(at(morning)));
    Ok(())
}

#[test]
fn unique_keeps_the_latest_visit_to_each_target() -> Result<(), String> {
    let store = FakeHistoryStore::default();
    let mut history = controller(&store);
    let (a, b) = (live("a"), vod("42"));
    let mut playback = playing(&a, PlaybackDetails::default());
    play_through(&mut history, &playback, &a, 0, 60)?;
    open(&mut playback, &b, PlaybackDetails::default());
    play_through(&mut history, &playback, &b, 61, 90)?;
    open(&mut playback, &a, PlaybackDetails::default());
    play_through(&mut history, &playback, &a, 91, 120)?;
    let unique: Vec<(&PlaybackTarget, Timestamp)> = history
        .unique()
        .into_iter()
        .map(|v| (&v.target, v.started_at))
        .collect();
    assert_eq!(unique, [(&a, at(91)), (&b, at(61))]);
    Ok(())
}

fn three_visits() -> (FakeHistoryStore, PlaybackTarget, PlaybackTarget) {
    let (a, b) = (live("a"), vod("42"));
    let visit = |id, target: &PlaybackTarget, start| Visit {
        id: VisitId::from_raw(id),
        target: target.clone(),
        started_at: at(start),
        last_seen_at: at(start + 60),
        details: VisitDetails::default(),
    };
    let store =
        FakeHistoryStore::with_visits(vec![visit(1, &a, 0), visit(2, &b, 100), visit(3, &a, 200)]);
    (store, a, b)
}

#[test]
fn removing_a_visit_keeps_the_others() -> Result<(), String> {
    let (store, a, b) = three_visits();
    let mut history = controller(&store);
    history.load().map_err(|e| e.to_string())?;
    history
        .remove(VisitId::from_raw(3))
        .map_err(|e| e.to_string())?;
    assert_eq!(targets(&store), [b.clone(), a.clone()]);
    let listed: Vec<&PlaybackTarget> = history.visits().iter().map(|v| &v.target).collect();
    assert_eq!(listed, [&b, &a]);
    Ok(())
}

#[test]
fn removing_a_target_removes_every_visit_to_it() -> Result<(), String> {
    let (store, a, b) = three_visits();
    let mut history = controller(&store);
    history.load().map_err(|e| e.to_string())?;
    history.remove_target(&a).map_err(|e| e.to_string())?;
    assert_eq!(targets(&store), [b]);
    assert_eq!(history.unique().len(), 1);
    Ok(())
}

#[test]
fn clearing_removes_everything() -> Result<(), String> {
    let (store, _, _) = three_visits();
    let mut history = controller(&store);
    history.load().map_err(|e| e.to_string())?;
    history.clear().map_err(|e| e.to_string())?;
    assert!(store.visits().is_empty());
    assert!(history.visits().is_empty());
    Ok(())
}

#[test]
fn a_failed_removal_leaves_the_list_as_it_was() -> Result<(), String> {
    let (store, _, _) = three_visits();
    let mut history = HistoryController::new(Box::new(store.clone().failing_writes()));
    history.load().map_err(|e| e.to_string())?;
    assert!(history.remove(VisitId::from_raw(1)).is_err());
    assert!(history.clear().is_err());
    assert_eq!(history.visits().len(), 3);
    assert_eq!(store.visits().len(), 3);
    Ok(())
}

#[test]
fn a_failed_write_stores_nothing_and_the_next_report_tries_again() -> Result<(), String> {
    let store = FakeHistoryStore::default();
    let mut failing = HistoryController::new(Box::new(store.clone().failing_writes()));
    let a = live("a");
    let playback = playing(&a, PlaybackDetails::default());
    play_through(&mut failing, &playback, &a, 0, secs(MIN_WATCH) - 1)?;
    assert!(failing.heard(&playback, &a, at(secs(MIN_WATCH))).is_err());
    assert!(
        failing
            .heard(&playback, &a, at(secs(MIN_WATCH) + 1))
            .is_err()
    );
    assert!(store.visits().is_empty());
    Ok(())
}

#[test]
fn a_visit_removed_while_it_plays_stays_removed() -> Result<(), String> {
    let store = FakeHistoryStore::default();
    let mut history = controller(&store);
    let a = live("a");
    let playback = playing(&a, PlaybackDetails::default());
    play_through(&mut history, &playback, &a, 0, secs(MIN_WATCH))?;
    // Another head clears the history.
    let mut other = controller(&store);
    other.clear().map_err(|e| e.to_string())?;

    let next = secs(MIN_WATCH) + secs(TOUCH_STEP);
    assert_eq!(
        history.heard(&playback, &a, at(next)),
        Ok(HistoryChange::Listed)
    );
    assert!(store.visits().is_empty());
    assert!(history.visits().is_empty());
    // Playing on is the next visit.
    play_through(
        &mut history,
        &playback,
        &a,
        next + 1,
        next + 1 + secs(MIN_WATCH),
    )?;
    assert_eq!(
        store.visits().first().map(|v| v.started_at),
        Some(at(next + 1))
    );
    Ok(())
}

#[test]
fn moving_on_writes_the_last_report_of_the_visit_left() -> Result<(), String> {
    let store = FakeHistoryStore::default();
    let mut history = controller(&store);
    let (a, b) = (live("a"), live("b"));
    let mut playback = playing(&a, PlaybackDetails::default());
    // Written at 10 and 40; heard until 55.
    play_through(&mut history, &playback, &a, 0, 55)?;
    open(&mut playback, &b, PlaybackDetails::default());
    play_through(&mut history, &playback, &b, 56, 56)?;
    assert_eq!(store.visits().first().map(|v| v.last_seen_at), Some(at(55)));
    Ok(())
}

#[test]
fn leaving_a_player_stopped_away_writes_its_last_report() -> Result<(), String> {
    let store = FakeHistoryStore::default();
    let mut history = controller(&store);
    let a = live("a");
    let mut playback = playing(&a, PlaybackDetails::default());
    play_through(&mut history, &playback, &a, 0, 55)?;
    playback.background(false, true, Seconds::ZERO);
    history
        .leave(&playback, &a, at(500))
        .map_err(|e| e.to_string())?;
    assert_eq!(store.visits().first().map(|v| v.last_seen_at), Some(at(55)));
    Ok(())
}

#[test]
fn rewinding_a_live_page_keeps_counting_its_visit() -> Result<(), String> {
    let store = FakeHistoryStore::default();
    let mut history = controller(&store);
    let a = live("a");
    let Ok(archive) = VideoId::parse("2001") else {
        unreachable!("valid id");
    };
    let mut playback = playing(
        &a,
        PlaybackDetails {
            live: Some(LiveDetails {
                viewers: 1,
                started_at: Some(at(0)),
                archive: Some(archive.clone()),
            }),
            ..titled("live")
        },
    );
    play_through(&mut history, &playback, &a, 100, 101)?;
    if playback.rewind(&a, at(3_600)).is_none() {
        return Err("the rewind is offered".to_owned());
    }
    assert!(playback.playing(&a));
    let vod = Playback::from_variants(
        "https://example.test/vod/master.m3u8".to_owned(),
        vec![Variant {
            name: "720p".to_owned(),
            url: "https://example.test/vod/720p.m3u8".to_owned(),
            resolution: None,
            frame_rate: None,
            bandwidth: 1000,
            kind: QualityKind::Source,
        }],
    );
    assert_eq!(
        playback.apply_rewind(
            &a,
            &archive,
            Ok(vod),
            Ok(titled("recording")),
            QualityPreference::Source,
        ),
        Some(RewindOutcome::Rewound)
    );
    assert!(playback.playing(&a));
    play_through(&mut history, &playback, &a, 102, 103)?;
    assert!(playback.back_to_live(&a));
    assert!(playback.playing(&a));
    play_through(&mut history, &playback, &a, 104, 100 + secs(MIN_WATCH))?;
    let visits = store.visits();
    let [visit] = visits.as_slice() else {
        return Err(format!("expected one visit, got {visits:?}"));
    };
    assert_eq!(visit.target, a);
    assert_eq!(visit.started_at, at(100));
    Ok(())
}
