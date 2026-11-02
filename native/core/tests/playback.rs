use std::time::{Duration, Instant};

use jitter_core::chat_replay::ChatReplay;
use jitter_core::fakes::FakeResumeStore;
use jitter_core::models::{
    BackgroundStep, CategoryId, ChannelLogin, Chapter, Chapters, FadeLevel, GameRef, LiveDetails,
    NoRewind, Open, Playback, PlaybackDetails, QualityKind, QualityPreference, Resolution,
    ResumePoint, ReturnOutcome, Rewind, RewindFailure, RewindOutcome, SLEEP_FADE, Seconds,
    SleepStep, Timestamp, Variant, VideoId,
};
use jitter_core::playback::{PlaybackController, PlaybackState, PlaybackTarget, TIMED_OUT};
use jitter_core::resume::ResumeController;
use jitter_core::resume_policy::ResumeDecision;
use jitter_core::rewind_policy::{REWIND_BEHIND, RewindPolicy};

fn live(login: &str) -> PlaybackTarget {
    let Ok(l) = ChannelLogin::parse(login) else {
        unreachable!("valid login");
    };
    PlaybackTarget::Live(l)
}

fn variant(name: &str, height: u32, kind: QualityKind) -> Variant {
    Variant {
        name: name.to_owned(),
        url: format!("https://example.test/{name}.m3u8"),
        resolution: Some(Resolution {
            width: height * 16 / 9,
            height,
        }),
        frame_rate: Some(30),
        bandwidth: 1000,
        kind,
    }
}

fn playback(variants: Vec<Variant>) -> Playback {
    Playback::from_variants("https://example.test/master.m3u8".to_owned(), variants)
}

fn no_details() -> Result<PlaybackDetails, String> {
    Ok(PlaybackDetails::default())
}

fn selected(c: &PlaybackController) -> Option<&str> {
    c.selected_quality().map(|q| q.name.as_str())
}

#[test]
fn result_for_another_target_is_ignored() {
    let mut c = PlaybackController::new();
    c.begin(live("a"));
    c.begin(live("b"));
    assert!(!c.apply(
        live("a"),
        Ok(playback(vec![variant("720p", 720, QualityKind::Transcode)])),
        no_details(),
        QualityPreference::Source
    ));
    assert_eq!(c.state(), &PlaybackState::Resolving { target: live("b") });
    assert!(!c.apply(
        live("a"),
        Err("late".to_owned()),
        no_details(),
        QualityPreference::Source
    ));
    assert_eq!(c.state(), &PlaybackState::Resolving { target: live("b") });
}

#[test]
fn result_while_idle_is_ignored() {
    let mut c = PlaybackController::new();
    c.apply(
        live("a"),
        Err("x".to_owned()),
        no_details(),
        QualityPreference::Source,
    );
    assert_eq!(c.state(), &PlaybackState::Idle);
}

fn timed_out() -> PlaybackState {
    PlaybackState::Failed {
        message: TIMED_OUT.to_owned(),
    }
}

#[test]
fn a_resolve_that_times_out_fails_and_its_late_result_is_ignored() {
    let mut c = PlaybackController::new();
    let attempt = c.begin(live("a"));
    assert!(c.time_out(attempt));
    assert_eq!(c.state(), &timed_out());
    assert!(!c.apply(
        live("a"),
        one_quality(),
        no_details(),
        QualityPreference::Source
    ));
    assert_eq!(c.state(), &timed_out());
}

#[test]
fn a_retry_gets_a_fresh_attempt_the_old_timeout_cannot_fail() {
    let mut c = PlaybackController::new();
    let first = c.begin(live("a"));
    let retry = c.begin(live("a"));
    assert!(!c.time_out(first));
    assert_eq!(c.state(), &PlaybackState::Resolving { target: live("a") });
    assert!(c.time_out(retry));
    assert_eq!(c.state(), &timed_out());
}

#[test]
fn a_timeout_after_the_answer_or_leaving_changes_nothing() {
    let mut c = PlaybackController::new();
    let attempt = c.begin(live("a"));
    assert!(c.apply(
        live("a"),
        one_quality(),
        no_details(),
        QualityPreference::Source
    ));
    assert!(!c.time_out(attempt));
    assert!(matches!(c.state(), PlaybackState::Ready { .. }));

    let attempt = c.begin(live("b"));
    assert!(c.leave(&live("b")));
    assert!(!c.time_out(attempt));
    assert_eq!(c.state(), &PlaybackState::Idle);
}

#[test]
fn source_is_selected_by_default() {
    let mut c = PlaybackController::new();
    c.begin(live("a"));
    c.apply(
        live("a"),
        Ok(playback(vec![
            variant("720p", 720, QualityKind::Transcode),
            variant("1080p (source)", 1080, QualityKind::Source),
        ])),
        no_details(),
        QualityPreference::Source,
    );
    assert_eq!(selected(&c), Some("1080p (source)"));
}

#[test]
fn no_source_selects_first_quality() {
    let mut c = PlaybackController::new();
    c.begin(live("a"));
    c.apply(
        live("a"),
        Ok(playback(vec![
            variant("480p", 480, QualityKind::Transcode),
            variant("720p", 720, QualityKind::Transcode),
        ])),
        no_details(),
        QualityPreference::Source,
    );
    assert_eq!(selected(&c), Some("720p"));
}

#[test]
fn select_unknown_name_keeps_selection_and_known_name_changes_it() {
    let mut c = PlaybackController::new();
    let Ok(id) = VideoId::parse("123") else {
        unreachable!("valid id");
    };
    c.begin(PlaybackTarget::Vod(id.clone()));
    c.apply(
        PlaybackTarget::Vod(id),
        Ok(playback(vec![
            variant("720p", 720, QualityKind::Transcode),
            variant("480p", 480, QualityKind::Transcode),
        ])),
        no_details(),
        QualityPreference::Source,
    );
    c.select("nope");
    assert_eq!(selected(&c), Some("720p"));
    c.select("480p");
    assert_eq!(selected(&c), Some("480p"));
}

#[test]
fn error_and_empty_playback_fail() {
    let mut c = PlaybackController::new();
    c.begin(live("a"));
    assert!(c.apply(
        live("a"),
        Err("offline".to_owned()),
        no_details(),
        QualityPreference::Source
    ));
    assert_eq!(
        c.state(),
        &PlaybackState::Failed {
            message: "offline".to_owned()
        }
    );
    c.begin(live("a"));
    c.apply(
        live("a"),
        Ok(playback(vec![])),
        no_details(),
        QualityPreference::Source,
    );
    assert!(matches!(c.state(), PlaybackState::Failed { .. }));
}

fn gta() -> PlaybackDetails {
    let Ok(id) = CategoryId::parse("32982") else {
        unreachable!("valid id");
    };
    PlaybackDetails {
        channel: None,
        title: Some("GTA RP".to_owned()),
        game: Some(GameRef {
            id,
            name: "Grand Theft Auto V".to_owned(),
        }),
        chapters: Chapters::default(),
        streamed_at: None,
        live: None,
    }
}

fn details(c: &PlaybackController) -> Option<&PlaybackDetails> {
    match c.state() {
        PlaybackState::Ready { details, .. } => Some(&**details),
        _ => None,
    }
}

fn one_quality() -> Result<Playback, String> {
    Ok(playback(vec![variant("720p", 720, QualityKind::Transcode)]))
}

#[test]
fn ready_carries_the_details() {
    let mut c = PlaybackController::new();
    c.begin(live("a"));
    assert!(c.apply(
        live("a"),
        one_quality(),
        Ok(gta()),
        QualityPreference::Source
    ));
    assert_eq!(details(&c), Some(&gta()));
}

#[test]
fn details_that_failed_to_load_still_play_without_them() {
    let mut c = PlaybackController::new();
    c.begin(live("a"));
    assert!(c.apply(
        live("a"),
        one_quality(),
        Err("timed out".to_owned()),
        QualityPreference::Source
    ));
    assert_eq!(selected(&c), Some("720p"));
    assert_eq!(details(&c), Some(&PlaybackDetails::default()));
}

#[test]
fn details_do_not_rescue_a_failed_resolve() {
    let mut c = PlaybackController::new();
    c.begin(live("a"));
    assert!(c.apply(
        live("a"),
        Err("offline".to_owned()),
        Ok(gta()),
        QualityPreference::Source
    ));
    assert_eq!(
        c.state(),
        &PlaybackState::Failed {
            message: "offline".to_owned()
        }
    );
}

#[test]
fn switching_quality_keeps_the_details() {
    let mut c = PlaybackController::new();
    c.begin(live("a"));
    c.apply(
        live("a"),
        Ok(playback(vec![
            variant("1080p60", 1080, QualityKind::Source),
            variant("720p60", 720, QualityKind::Transcode),
        ])),
        Ok(gta()),
        QualityPreference::Source,
    );
    c.switch_quality("720p60", Seconds::ZERO);
    assert_eq!(details(&c), Some(&gta()));
}

fn vod() -> VideoId {
    let Ok(id) = VideoId::parse("123") else {
        unreachable!("valid id");
    };
    id
}

#[test]
fn a_vod_carries_when_it_was_streamed() {
    let streamed = PlaybackDetails {
        channel: None,
        title: None,
        game: None,
        chapters: Chapters::default(),
        streamed_at: Some(Timestamp::from_unix(1_792_875_046)),
        live: None,
    };
    let mut c = PlaybackController::new();
    c.begin(PlaybackTarget::Vod(vod()));
    assert!(c.apply(
        PlaybackTarget::Vod(vod()),
        one_quality(),
        Ok(streamed.clone()),
        QualityPreference::Source
    ));
    assert_eq!(details(&c), Some(&streamed));
}

fn game(id: &str, name: &str) -> GameRef {
    let Ok(id) = CategoryId::parse(id) else {
        unreachable!("valid id");
    };
    GameRef {
        id,
        name: name.to_owned(),
    }
}

/// A VOD in Just Chatting for its first hour, then in GTA V.
fn chaptered() -> PlaybackDetails {
    PlaybackDetails {
        game: Some(game("509658", "Just Chatting")),
        chapters: Chapters::new(vec![
            Chapter {
                start: Seconds::new(3_600),
                game: game("32982", "Grand Theft Auto V"),
            },
            Chapter {
                start: Seconds::ZERO,
                game: game("509658", "Just Chatting"),
            },
        ]),
        ..PlaybackDetails::default()
    }
}

fn chaptered_vod() -> PlaybackController {
    let mut c = PlaybackController::new();
    c.begin(PlaybackTarget::Vod(vod()));
    c.apply(
        PlaybackTarget::Vod(vod()),
        one_quality(),
        Ok(chaptered()),
        QualityPreference::Source,
    );
    c
}

/// The game the page shows, as the player last reported its position.
fn game_shown(c: &PlaybackController) -> Option<&str> {
    match c.state() {
        PlaybackState::Ready { details, at, .. } => details.game_at(*at).map(|g| g.name.as_str()),
        _ => None,
    }
}

#[test]
fn chapters_tell_the_game_at_a_position_in_any_order() {
    let chapters = chaptered().chapters;
    let at = |s| chapters.game_at(Seconds::new(s)).map(|g| g.name.as_str());
    assert_eq!(at(0), Some("Just Chatting"));
    assert_eq!(at(3_599), Some("Just Chatting"));
    assert_eq!(at(3_600), Some("Grand Theft Auto V"));
    assert_eq!(at(90_000), Some("Grand Theft Auto V"));
    assert_eq!(Chapters::default().game_at(Seconds::ZERO), None);
}

#[test]
fn a_chapter_is_gone_to_at_its_start() {
    let c = chaptered_vod();
    assert_eq!(c.chapter_start(0), Some(Seconds::ZERO));
    assert_eq!(c.chapter_start(1), Some(Seconds::new(3_600)));
    assert_eq!(c.chapter_start(2), None);
    assert_eq!(ready(live("a")).chapter_start(0), None);
    assert_eq!(PlaybackController::new().chapter_start(0), None);
}

#[test]
fn only_a_vod_in_two_chapters_or_more_has_chapters_to_go_to() {
    assert_eq!(chaptered().chapters.to_go_to().len(), 2);
    let one = Chapters::new(vec![Chapter {
        start: Seconds::ZERO,
        game: game("509658", "Just Chatting"),
    }]);
    assert!(one.to_go_to().is_empty());
    assert!(Chapters::default().to_go_to().is_empty());
}

#[test]
fn before_its_first_chapter_a_vod_is_in_that_chapter_s_game() {
    let late = Chapters::new(vec![Chapter {
        start: Seconds::new(30),
        game: game("32982", "Grand Theft Auto V"),
    }]);
    assert_eq!(
        late.game_at(Seconds::ZERO).map(|g| g.name.as_str()),
        Some("Grand Theft Auto V")
    );
}

#[test]
fn a_chaptered_vod_shows_the_game_at_the_player_s_position() {
    let mut c = chaptered_vod();
    let target = PlaybackTarget::Vod(vod());
    assert_eq!(game_shown(&c), None, "no position yet");

    assert!(c.follow(&target, Seconds::new(10), Seconds::new(7_200)));
    assert_eq!(game_shown(&c), Some("Just Chatting"));
    assert!(!c.follow(&target, Seconds::new(11), Seconds::new(7_200)));

    assert!(c.follow(&target, Seconds::new(3_600), Seconds::new(7_200)));
    assert_eq!(game_shown(&c), Some("Grand Theft Auto V"));

    assert!(
        c.follow(&target, Seconds::new(5), Seconds::new(7_200)),
        "seeked back"
    );
    assert_eq!(game_shown(&c), Some("Just Chatting"));
}

#[test]
fn a_vod_without_chapters_shows_its_game_from_the_start() {
    let mut c = PlaybackController::new();
    c.begin(PlaybackTarget::Vod(vod()));
    c.apply(
        PlaybackTarget::Vod(vod()),
        one_quality(),
        Ok(gta()),
        QualityPreference::Source,
    );
    assert_eq!(game_shown(&c), Some("Grand Theft Auto V"));
    assert!(!c.follow(
        &PlaybackTarget::Vod(vod()),
        Seconds::new(4_000),
        Seconds::new(7_200)
    ));
    assert_eq!(game_shown(&c), Some("Grand Theft Auto V"));
}

#[test]
fn an_unloaded_stale_or_stopped_player_s_position_is_not_followed() {
    let mut c = chaptered_vod();
    let target = PlaybackTarget::Vod(vod());
    assert!(
        !c.follow(&target, Seconds::new(4_000), Seconds::ZERO),
        "not loaded"
    );
    assert!(!c.follow(&live("a"), Seconds::new(4_000), Seconds::new(7_200)));
    assert_eq!(game_shown(&c), None);

    c.background(false, true, Seconds::ZERO);
    assert!(!c.follow(&target, Seconds::new(4_000), Seconds::new(7_200)));
    assert_eq!(game_shown(&c), None);
}

fn ready(target: PlaybackTarget) -> PlaybackController {
    let mut c = PlaybackController::new();
    c.begin(target.clone());
    c.apply(
        target,
        Ok(playback(vec![
            variant("1080p60", 1080, QualityKind::Source),
            variant("720p60", 720, QualityKind::Transcode),
        ])),
        no_details(),
        QualityPreference::Source,
    );
    c
}

fn resume_at(position: u32, duration: u32) -> ResumeController {
    ResumeController::new(Box::new(FakeResumeStore::with_points(vec![ResumePoint {
        video_id: vod(),
        position: Seconds::new(position),
        duration: Seconds::new(duration),
        saved_at: Timestamp::from_unix(0),
    }])))
}

fn open(name: &str, start: u32, live: bool) -> Open {
    Open {
        url: format!("https://example.test/{name}.m3u8"),
        start: Seconds::new(start),
        live,
    }
}

#[test]
fn default_alias_selects_the_default_quality() {
    let mut c = ready(live("a"));
    assert!(c.select("720p60"));
    assert!(c.select(Playback::DEFAULT_QUALITY));
    assert_eq!(selected(&c), Some("1080p60"));
}

#[test]
fn default_alias_without_a_source_selects_the_first_quality() {
    let p = playback(vec![
        variant("480p30", 480, QualityKind::Transcode),
        variant("720p30", 720, QualityKind::Transcode),
    ]);
    assert_eq!(
        p.quality(Playback::DEFAULT_QUALITY)
            .map(|q| q.name.as_str()),
        Some("720p30")
    );
}

#[test]
fn a_real_quality_named_like_the_alias_wins() {
    let p = playback(vec![
        variant("1080p60", 1080, QualityKind::Source),
        variant(Playback::DEFAULT_QUALITY, 720, QualityKind::Transcode),
    ]);
    assert_eq!(
        p.quality(Playback::DEFAULT_QUALITY)
            .map(|q| q.name.as_str()),
        Some(Playback::DEFAULT_QUALITY)
    );
}

#[test]
fn select_reports_unknown_names() {
    let mut c = ready(live("a"));
    assert!(!c.select("nope"));
    assert!(!PlaybackController::new().select(Playback::DEFAULT_QUALITY));
}

#[test]
fn vod_opens_at_its_stored_point() {
    let c = ready(PlaybackTarget::Vod(vod()));
    assert_eq!(
        c.open(&resume_at(600, 3600)),
        Ok(Some(open("1080p60", 600, false)))
    );
}

#[test]
fn vod_without_a_point_opens_at_zero() {
    let c = ready(PlaybackTarget::Vod(vod()));
    let resume = ResumeController::new(Box::new(FakeResumeStore::default()));
    assert_eq!(c.open(&resume), Ok(Some(open("1080p60", 0, false))));
}

#[test]
fn live_opens_at_zero() {
    let c = ready(live("a"));
    assert_eq!(
        c.open(&resume_at(600, 3600)),
        Ok(Some(open("1080p60", 0, true)))
    );
}

#[test]
fn open_before_ready_is_none_and_a_failed_load_is_err() {
    let mut c = PlaybackController::new();
    assert_eq!(c.open(&resume_at(600, 3600)), Ok(None));
    c = ready(PlaybackTarget::Vod(vod()));
    let resume = ResumeController::new(Box::new(FakeResumeStore::default().failing_loads()));
    assert!(c.open(&resume).is_err());
}

#[test]
fn quality_switch_keeps_the_position() {
    let mut c = ready(PlaybackTarget::Vod(vod()));
    assert_eq!(
        c.switch_quality("720p60", Seconds::new(1234)),
        Some(open("720p60", 1234, false))
    );
    assert_eq!(selected(&c), Some("720p60"));
}

#[test]
fn live_quality_switch_opens_at_the_live_edge() {
    let mut c = ready(live("a"));
    assert_eq!(
        c.switch_quality("720p60", Seconds::new(1234)),
        Some(open("720p60", 0, true))
    );
}

#[test]
fn switch_to_unknown_quality_changes_nothing() {
    let mut c = ready(PlaybackTarget::Vod(vod()));
    assert_eq!(c.switch_quality("nope", Seconds::new(1234)), None);
    assert_eq!(selected(&c), Some("1080p60"));
}

fn now() -> Timestamp {
    Timestamp::from_unix(1_000)
}

fn record(
    c: &PlaybackController,
    resume: &mut ResumeController,
    target: &PlaybackTarget,
    position: u32,
    duration: u32,
) -> Option<ResumeDecision> {
    let Ok(decision) = c.record(
        resume,
        target,
        Seconds::new(position),
        Seconds::new(duration),
        now(),
    ) else {
        unreachable!("fake store does not fail");
    };
    decision
}

#[test]
fn the_playing_vod_records_its_position() {
    let store = FakeResumeStore::default();
    let mut resume = ResumeController::new(Box::new(store.clone()));
    let target = PlaybackTarget::Vod(vod());
    let c = ready(target.clone());
    assert_eq!(
        record(&c, &mut resume, &target, 600, 3600),
        Some(ResumeDecision::Save)
    );
    assert_eq!(
        store.points(),
        vec![ResumePoint {
            video_id: vod(),
            position: Seconds::new(600),
            duration: Seconds::new(3600),
            saved_at: now(),
        }]
    );
}

#[test]
fn live_another_target_or_an_unloaded_player_records_nothing() {
    let store = FakeResumeStore::default();
    let mut resume = ResumeController::new(Box::new(store.clone()));
    let target = PlaybackTarget::Vod(vod());
    let c = ready(live("a"));
    assert_eq!(record(&c, &mut resume, &live("a"), 600, 3600), None);
    let c = ready(target.clone());
    let Ok(other) = VideoId::parse("456") else {
        unreachable!("valid id");
    };
    assert_eq!(
        record(&c, &mut resume, &PlaybackTarget::Vod(other), 600, 3600),
        None
    );
    assert_eq!(record(&c, &mut resume, &target, 0, 0), None);
    let mut c = PlaybackController::new();
    c.begin(target.clone());
    assert_eq!(record(&c, &mut resume, &target, 600, 3600), None);
    assert_eq!(store.points(), vec![]);
}

#[test]
fn leaving_goes_idle() {
    let mut c = ready(live("a"));
    assert!(c.leave(&live("a")));
    assert_eq!(c.state(), &PlaybackState::Idle);
    c.begin(live("a"));
    c.apply(
        live("a"),
        Err("offline".to_owned()),
        no_details(),
        QualityPreference::Source,
    );
    assert!(c.leave(&live("a")));
    assert_eq!(c.state(), &PlaybackState::Idle);
}

#[test]
fn leaving_a_replaced_target_keeps_the_new_one() {
    let mut c = ready(live("a"));
    c.begin(live("b"));
    assert!(!c.leave(&live("a")));
    assert_eq!(c.state(), &PlaybackState::Resolving { target: live("b") });
    let mut c = ready(live("b"));
    assert!(!c.leave(&live("a")));
    assert!(matches!(c.state(), PlaybackState::Ready { .. }));
}

// Sleep timer

fn minutes(m: u64) -> Duration {
    Duration::from_secs(m * 60)
}

fn sleep_counting(relabeled: bool) -> SleepStep {
    SleepStep::Counting {
        relabeled,
        fade: None,
    }
}

#[test]
fn a_sleep_timer_arms_only_while_ready() {
    let t0 = Instant::now();
    let mut c = PlaybackController::new();
    assert!(!c.arm_sleep(15, t0));
    c.begin(live("a"));
    assert!(!c.arm_sleep(15, t0));
    assert_eq!(c.sleep_wake(), None);
    let mut c = ready(live("a"));
    assert!(!c.arm_sleep(0, t0));
    assert!(c.arm_sleep(15, t0));
    assert_eq!(c.sleep_timer().map(|t| t.minutes_left()), Some(15));
    assert_eq!(c.sleep_wake(), Some(t0 + minutes(1)));
}

#[test]
fn arming_again_replaces_the_timer() {
    let t0 = Instant::now();
    let mut c = ready(live("a"));
    c.arm_sleep(90, t0);
    c.arm_sleep(15, t0 + minutes(5));
    assert_eq!(c.sleep_timer().map(|t| t.minutes_left()), Some(15));
    assert_eq!(c.sleep_tick(t0 + minutes(19)), Some(sleep_counting(true)));
    assert_eq!(c.sleep_tick(t0 + minutes(20)), Some(SleepStep::Expired));
}

#[test]
fn a_turned_off_timer_never_ticks() {
    let t0 = Instant::now();
    let mut c = ready(live("a"));
    assert!(!c.turn_off_sleep());
    c.arm_sleep(15, t0);
    assert!(c.turn_off_sleep());
    assert_eq!(c.sleep_wake(), None);
    assert_eq!(c.sleep_tick(t0 + minutes(15)), None);
    assert!(matches!(
        c.state(),
        PlaybackState::Ready { sleep: None, .. }
    ));
}

#[test]
fn a_quality_switch_keeps_the_timer() {
    let mut c = ready(live("a"));
    c.arm_sleep(15, Instant::now());
    assert!(c.switch_quality("720p60", Seconds::ZERO).is_some());
    assert!(c.sleep_timer().is_some());
}

#[test]
fn the_timer_fades_then_stops_the_page_it_was_armed_on() {
    let t0 = Instant::now();
    let mut c = ready(live("a"));
    c.arm_sleep(1, t0);
    let end = t0 + minutes(1);
    assert_eq!(
        c.sleep_tick(end - SLEEP_FADE),
        Some(SleepStep::Counting {
            relabeled: false,
            fade: Some(FadeLevel::FULL)
        })
    );
    assert_eq!(
        c.sleep_tick(end - SLEEP_FADE / 2),
        Some(SleepStep::Counting {
            relabeled: false,
            fade: Some(FadeLevel::new(0.5))
        })
    );
    assert_eq!(c.sleep_tick(end), Some(SleepStep::Expired));
    assert_eq!(c.state(), &PlaybackState::Slept { target: live("a") });
    assert_eq!(c.sleep_wake(), None);
    assert_eq!(c.sleep_tick(end + minutes(1)), None);
}

#[test]
fn a_slept_page_plays_nothing_and_records_nothing() {
    let store = FakeResumeStore::default();
    let mut resume = ResumeController::new(Box::new(store.clone()));
    let target = PlaybackTarget::Vod(vod());
    let t0 = Instant::now();
    let mut c = ready(target.clone());
    c.arm_sleep(1, t0);
    c.sleep_tick(t0 + minutes(1));
    assert_eq!(c.open(&resume), Ok(None));
    assert!(!c.select("720p60"));
    assert_eq!(record(&c, &mut resume, &target, 0, 0), None);
    assert_eq!(record(&c, &mut resume, &target, 600, 3600), None);
    assert_eq!(store.points(), vec![]);
}

#[test]
fn resuming_a_slept_page_plays_it_again_without_a_timer() {
    let t0 = Instant::now();
    let mut c = ready(live("a"));
    c.arm_sleep(1, t0);
    c.sleep_tick(t0 + minutes(1));
    c.begin(live("a"));
    assert_eq!(c.state(), &PlaybackState::Resolving { target: live("a") });
    c.apply(
        live("a"),
        one_quality(),
        no_details(),
        QualityPreference::Source,
    );
    assert!(matches!(
        c.state(),
        PlaybackState::Ready { sleep: None, .. }
    ));
}

#[test]
fn leaving_or_playing_another_target_cancels_the_timer() {
    let t0 = Instant::now();
    let mut c = ready(live("a"));
    c.arm_sleep(15, t0);
    c.begin(live("b"));
    assert_eq!(c.sleep_wake(), None);
    assert_eq!(c.sleep_tick(t0 + minutes(15)), None);
    let mut c = ready(live("a"));
    c.arm_sleep(15, t0);
    assert!(c.leave(&live("a")));
    assert_eq!(c.sleep_wake(), None);
}

#[test]
fn leaving_a_slept_page_goes_idle_unless_it_was_replaced() {
    let t0 = Instant::now();
    let mut c = ready(live("a"));
    c.arm_sleep(1, t0);
    c.sleep_tick(t0 + minutes(1));
    assert!(!c.leave(&live("b")));
    assert_eq!(c.state(), &PlaybackState::Slept { target: live("a") });
    assert!(c.leave(&live("a")));
    assert_eq!(c.state(), &PlaybackState::Idle);
}

fn video(name: &str, height: u32, fps: u32, kind: QualityKind) -> Variant {
    Variant {
        frame_rate: Some(fps),
        ..variant(name, height, kind)
    }
}

fn audio_only() -> Variant {
    Variant {
        resolution: None,
        frame_rate: None,
        ..variant("audio_only", 0, QualityKind::AudioOnly)
    }
}

/// A typical live ladder: source 1080p60, then 720p60, 720p30, 480p, 160p
/// and audio only.
fn ladder() -> Playback {
    playback(vec![
        video("1080p60", 1080, 60, QualityKind::Source),
        video("720p60", 720, 60, QualityKind::Transcode),
        video("720p30", 720, 30, QualityKind::Transcode),
        video("480p30", 480, 30, QualityKind::Transcode),
        video("160p30", 160, 30, QualityKind::Transcode),
        audio_only(),
    ])
}

fn preferred(p: &Playback, preference: QualityPreference) -> Option<&str> {
    p.preferred_quality(preference).map(|q| q.name.as_str())
}

#[test]
fn a_preferred_height_the_stream_has_is_picked_at_its_best_frame_rate() {
    let p = ladder();
    assert_eq!(preferred(&p, QualityPreference::P1080), Some("1080p60"));
    assert_eq!(preferred(&p, QualityPreference::P720), Some("720p60"));
    assert_eq!(preferred(&p, QualityPreference::P480), Some("480p30"));
    assert_eq!(preferred(&p, QualityPreference::P160), Some("160p30"));
}

#[test]
fn a_missing_height_falls_to_the_closest_lower_one() {
    assert_eq!(
        preferred(&ladder(), QualityPreference::P360),
        Some("160p30")
    );
}

#[test]
fn nothing_low_enough_falls_back_to_the_best() {
    let p = playback(vec![
        video("1080p60", 1080, 60, QualityKind::Source),
        video("720p60", 720, 60, QualityKind::Transcode),
    ]);
    assert_eq!(preferred(&p, QualityPreference::P480), Some("1080p60"));
    assert_eq!(preferred(&p, QualityPreference::AudioOnly), Some("1080p60"));
}

#[test]
fn source_and_audio_only_pick_their_own_kind() {
    let p = ladder();
    assert_eq!(preferred(&p, QualityPreference::Source), Some("1080p60"));
    assert_eq!(
        preferred(&p, QualityPreference::AudioOnly),
        Some("audio_only")
    );
}

#[test]
fn a_source_below_the_preference_is_the_closest_lower() {
    let p = playback(vec![
        video("900p60", 900, 60, QualityKind::Source),
        video("720p30", 720, 30, QualityKind::Transcode),
    ]);
    assert_eq!(preferred(&p, QualityPreference::P1080), Some("900p60"));
}

#[test]
fn the_page_opens_at_the_preferred_quality_and_a_manual_switch_leaves_it() {
    let mut c = PlaybackController::new();
    c.begin(live("a"));
    assert!(c.apply(
        live("a"),
        Ok(ladder()),
        no_details(),
        QualityPreference::P480
    ));
    assert_eq!(selected(&c), Some("480p30"));
    assert!(c.switch_quality("1080p60", Seconds::ZERO).is_some());
    assert_eq!(selected(&c), Some("1080p60"));
}

fn ready_with_audio(target: PlaybackTarget) -> PlaybackController {
    let mut c = PlaybackController::new();
    c.begin(target.clone());
    c.apply(
        target,
        Ok(playback(vec![
            variant("1080p60", 1080, QualityKind::Source),
            variant("720p60", 720, QualityKind::Transcode),
            variant("audio_only", 0, QualityKind::AudioOnly),
        ])),
        no_details(),
        QualityPreference::Source,
    );
    c
}

fn no_resume() -> ResumeController {
    ResumeController::new(Box::new(FakeResumeStore::default()))
}

fn foreground(
    c: &mut PlaybackController,
    position: u32,
    resume: &ResumeController,
) -> Option<Open> {
    let Ok(open) = c.foreground(Seconds::new(position), resume) else {
        unreachable!("fake store does not fail");
    };
    open
}

#[test]
fn background_audio_carries_on_audio_only_and_comes_back_to_the_selection() {
    let mut c = ready_with_audio(PlaybackTarget::Vod(vod()));
    c.select("720p60");
    assert_eq!(
        c.background(true, true, Seconds::new(600)),
        Some(BackgroundStep::Open(open("audio_only", 600, false)))
    );
    assert_eq!(selected(&c), Some("720p60"));
    assert_eq!(
        foreground(&mut c, 900, &no_resume()),
        Some(open("720p60", 900, false))
    );
}

#[test]
fn background_audio_on_live_reopens_at_the_live_edge() {
    let mut c = ready_with_audio(live("a"));
    assert_eq!(
        c.background(true, true, Seconds::new(600)),
        Some(BackgroundStep::Open(open("audio_only", 0, true)))
    );
    assert_eq!(
        foreground(&mut c, 900, &no_resume()),
        Some(open("1080p60", 0, true))
    );
}

#[test]
fn already_audio_only_changes_nothing_either_way() {
    let mut c = ready_with_audio(live("a"));
    c.select("audio_only");
    assert_eq!(c.background(true, true, Seconds::ZERO), None);
    assert_eq!(foreground(&mut c, 0, &no_resume()), None);
}

#[test]
fn a_stream_without_audio_only_keeps_its_quality_in_the_background() {
    let mut c = ready(live("a"));
    assert_eq!(c.background(true, true, Seconds::ZERO), None);
    assert_eq!(foreground(&mut c, 0, &no_resume()), None);
}

#[test]
fn background_audio_off_stops_and_carries_on_where_it_stopped() {
    let mut c = ready_with_audio(PlaybackTarget::Vod(vod()));
    assert_eq!(
        c.background(false, true, Seconds::new(600)),
        Some(BackgroundStep::Stop)
    );
    assert!(c.stopped_away());
    assert_eq!(
        foreground(&mut c, 0, &no_resume()),
        Some(open("1080p60", 600, false))
    );
    assert!(!c.stopped_away());
}

#[test]
fn a_paused_player_stays_as_it_is_in_the_background_and_back() {
    for audio in [true, false] {
        let mut c = ready_with_audio(PlaybackTarget::Vod(vod()));
        assert_eq!(c.background(audio, false, Seconds::new(600)), None);
        assert!(!c.stopped_away());
        assert_eq!(foreground(&mut c, 600, &no_resume()), None);
    }
}

#[test]
fn played_again_away_a_paused_player_carries_on_audio_only() {
    let mut c = ready_with_audio(PlaybackTarget::Vod(vod()));
    c.background(true, false, Seconds::new(600));
    assert_eq!(
        c.play_away(true, Seconds::new(610)),
        Some(open("audio_only", 610, false))
    );
    assert_eq!(c.play_away(true, Seconds::new(620)), None);
    assert_eq!(
        foreground(&mut c, 900, &no_resume()),
        Some(open("1080p60", 900, false))
    );
}

#[test]
fn played_again_away_without_background_audio_plays_on_as_it_is() {
    let mut c = ready_with_audio(PlaybackTarget::Vod(vod()));
    c.background(false, false, Seconds::new(600));
    assert_eq!(c.play_away(false, Seconds::new(610)), None);
    assert!(!c.stopped_away());
    assert_eq!(foreground(&mut c, 900, &no_resume()), None);
}

#[test]
fn playing_away_changes_nothing_unless_paused_away() {
    let mut c = ready_with_audio(live("a"));
    assert_eq!(c.play_away(true, Seconds::ZERO), None);
    c.background(true, true, Seconds::ZERO);
    assert_eq!(c.play_away(true, Seconds::ZERO), None);
}

/// A page still resolving reports a player not playing; it is not one
/// the user paused.
#[test]
fn a_page_resolving_as_the_app_leaves_still_opens_audio_only() {
    let target = live("a");
    let mut c = PlaybackController::new();
    c.begin(target.clone());
    assert_eq!(c.background(true, false, Seconds::ZERO), None);
    c.apply(
        target,
        Ok(playback(vec![
            variant("1080p60", 1080, QualityKind::Source),
            variant("audio_only", 0, QualityKind::AudioOnly),
        ])),
        no_details(),
        QualityPreference::Source,
    );
    assert_eq!(c.open(&no_resume()), Ok(Some(open("audio_only", 0, true))));
}

#[test]
fn a_stopped_player_records_nothing() {
    let store = FakeResumeStore::default();
    let mut resume = ResumeController::new(Box::new(store.clone()));
    let target = PlaybackTarget::Vod(vod());
    let mut c = ready_with_audio(target.clone());
    c.background(false, true, Seconds::new(600));
    assert_eq!(record(&c, &mut resume, &target, 0, 3600), None);
    assert!(store.points().is_empty());
}

#[test]
fn a_listening_player_records_its_position() {
    let mut resume = no_resume();
    let target = PlaybackTarget::Vod(vod());
    let mut c = ready_with_audio(target.clone());
    c.background(true, true, Seconds::new(600));
    assert_eq!(
        record(&c, &mut resume, &target, 700, 3600),
        Some(ResumeDecision::Save)
    );
}

#[test]
fn backgrounding_twice_changes_nothing_the_second_time() {
    let mut c = ready_with_audio(live("a"));
    assert!(c.background(true, true, Seconds::ZERO).is_some());
    assert_eq!(c.background(false, true, Seconds::ZERO), None);
    assert!(!c.stopped_away());
}

#[test]
fn foreground_without_background_changes_nothing() {
    let mut c = ready_with_audio(live("a"));
    assert_eq!(foreground(&mut c, 0, &no_resume()), None);
}

#[test]
fn nothing_playing_has_nothing_to_change() {
    let mut c = PlaybackController::new();
    c.begin(live("a"));
    assert_eq!(c.background(true, true, Seconds::ZERO), None);
    assert_eq!(foreground(&mut c, 0, &no_resume()), None);
}

#[test]
fn a_page_resolved_while_listening_opens_audio_only() {
    let target = PlaybackTarget::Vod(vod());
    let mut c = PlaybackController::new();
    c.begin(target.clone());
    c.background(true, true, Seconds::ZERO);
    c.apply(
        target,
        Ok(playback(vec![
            variant("1080p60", 1080, QualityKind::Source),
            variant("audio_only", 0, QualityKind::AudioOnly),
        ])),
        no_details(),
        QualityPreference::Source,
    );
    let resume = resume_at(300, 3600);
    assert_eq!(c.open(&resume), Ok(Some(open("audio_only", 300, false))));
    assert_eq!(selected(&c), Some("1080p60"));
    assert_eq!(
        foreground(&mut c, 310, &resume),
        Some(open("1080p60", 310, false))
    );
}

#[test]
fn a_page_resolved_while_stopped_opens_nothing_until_back_then_at_its_resume_point() {
    let target = PlaybackTarget::Vod(vod());
    let mut c = PlaybackController::new();
    c.begin(target.clone());
    assert_eq!(c.background(false, true, Seconds::ZERO), None);
    c.apply(
        target,
        Ok(playback(vec![variant(
            "1080p60",
            1080,
            QualityKind::Source,
        )])),
        no_details(),
        QualityPreference::Source,
    );
    let resume = resume_at(300, 3600);
    assert_eq!(c.open(&resume), Ok(None));
    assert_eq!(
        foreground(&mut c, 0, &resume),
        Some(open("1080p60", 300, false))
    );
}

// Rewinding a live page into its recording VOD, and back.

const STARTED: i64 = 1_800_000_000;

/// The moment the user rewinds: an hour into the stream.
fn an_hour_in() -> Timestamp {
    Timestamp::from_unix(STARTED + 3_600)
}

fn video_id(raw: &str) -> VideoId {
    let Ok(id) = VideoId::parse(raw) else {
        unreachable!("valid id");
    };
    id
}

/// The recording VOD of the stream on `live("a")`.
fn archive() -> VideoId {
    video_id("2001")
}

fn live_details(archive: Option<VideoId>, started_at: Option<Timestamp>) -> PlaybackDetails {
    PlaybackDetails {
        title: Some("live title".to_owned()),
        live: Some(LiveDetails {
            viewers: 10,
            started_at,
            archive,
        }),
        ..PlaybackDetails::default()
    }
}

/// Live details of a stream started at [`STARTED`], recording into `archive`.
fn recording(archive: VideoId) -> Result<PlaybackDetails, String> {
    Ok(live_details(
        Some(archive),
        Some(Timestamp::from_unix(STARTED)),
    ))
}

/// The live stream's qualities: source, 720p60 and audio only.
fn live_playback() -> Playback {
    playback(vec![
        variant("1080p60", 1080, QualityKind::Source),
        variant("720p60", 720, QualityKind::Transcode),
        variant("audio_only", 0, QualityKind::AudioOnly),
    ])
}

/// A VOD quality: its URL tells it from the live one of the same name.
fn vod_variant(name: &str, height: u32, kind: QualityKind) -> Variant {
    Variant {
        url: format!("https://example.test/vod/{name}.m3u8"),
        ..variant(name, height, kind)
    }
}

fn vod_playback(variants: Vec<Variant>) -> Playback {
    Playback::from_variants("https://example.test/vod/master.m3u8".to_owned(), variants)
}

/// The recording VOD's qualities, the same names as live.
fn vod_ladder() -> Playback {
    vod_playback(vec![
        vod_variant("1080p60", 1080, QualityKind::Source),
        vod_variant("720p60", 720, QualityKind::Transcode),
        vod_variant("audio_only", 0, QualityKind::AudioOnly),
    ])
}

fn vod_open(name: &str, start: u32) -> Open {
    Open {
        url: format!("https://example.test/vod/{name}.m3u8"),
        start: Seconds::new(start),
        live: false,
    }
}

/// Where a rewind an hour in starts: [`REWIND_BEHIND`] before it.
const START: u32 = 3_580;

fn live_page_with(details: Result<PlaybackDetails, String>) -> PlaybackController {
    let mut c = PlaybackController::new();
    c.begin(live("a"));
    c.apply(
        live("a"),
        Ok(live_playback()),
        details,
        QualityPreference::Source,
    );
    c
}

/// Live page `a`, its recording offered.
fn live_page() -> PlaybackController {
    live_page_with(recording(archive()))
}

/// Live page `a` loading its recording an hour in.
fn loading() -> PlaybackController {
    let mut c = live_page();
    assert_eq!(c.rewind(&live("a"), an_hour_in()), Some(archive()));
    c
}

fn apply_rewind(
    c: &mut PlaybackController,
    result: Result<Playback, RewindFailure>,
) -> Option<RewindOutcome> {
    c.apply_rewind(
        &live("a"),
        &archive(),
        result,
        Ok(chaptered()),
        QualityPreference::Source,
    )
}

/// Live page `a` playing its recording from [`START`].
fn rewound() -> PlaybackController {
    let mut c = loading();
    assert_eq!(
        apply_rewind(&mut c, Ok(vod_ladder())),
        Some(RewindOutcome::Rewound)
    );
    c
}

/// Live page `a` playing its recording while going back to live.
fn returning() -> PlaybackController {
    let mut c = rewound();
    assert!(c.back_to_live(&live("a")));
    c
}

fn rewind_of(c: &PlaybackController) -> Option<&Rewind> {
    match c.state() {
        PlaybackState::Ready { rewind, .. } => rewind.as_ref(),
        _ => None,
    }
}

fn opened(c: &PlaybackController) -> Option<Open> {
    let Ok(open) = c.open(&no_resume()) else {
        unreachable!("fake store does not fail");
    };
    open
}

fn at_of(c: &PlaybackController) -> Option<Seconds> {
    match c.state() {
        PlaybackState::Ready { at, .. } => *at,
        _ => None,
    }
}

fn apply_return(
    c: &mut PlaybackController,
    result: Result<Playback, String>,
    details: Result<PlaybackDetails, String>,
) -> Option<ReturnOutcome> {
    c.apply_return(
        &live("a"),
        &archive(),
        result,
        details,
        QualityPreference::Source,
    )
}

#[test]
fn a_live_page_offers_the_recording_twitch_shows() {
    assert_eq!(
        rewind_of(&live_page()),
        Some(&Rewind::Available { video: archive() })
    );
}

#[test]
fn a_live_page_without_a_recording_a_start_or_details_offers_no_rewind_saying_why() {
    let started = Some(Timestamp::from_unix(STARTED));
    let cases = [
        (Ok(live_details(None, started)), NoRewind::NoRecording),
        (Ok(live_details(Some(archive()), None)), NoRewind::NoStart),
        (Err("timed out".to_owned()), NoRewind::NoDetails),
        (Ok(PlaybackDetails::default()), NoRewind::NoDetails),
    ];
    for (details, why) in cases {
        let c = live_page_with(details);
        assert_eq!(rewind_of(&c), Some(&Rewind::Unavailable(why)));
    }
}

#[test]
fn the_recording_twitch_refused_is_not_offered() {
    assert_eq!(
        RewindPolicy::offer(&recording(archive()), Some(&archive())),
        Rewind::Unavailable(NoRewind::Forbidden)
    );
    assert_eq!(
        RewindPolicy::offer(&recording(archive()), Some(&vod())),
        Rewind::Available { video: archive() }
    );
}

#[test]
fn a_vod_page_has_no_rewind() {
    let mut c = PlaybackController::new();
    c.begin(PlaybackTarget::Vod(vod()));
    c.apply(
        PlaybackTarget::Vod(vod()),
        one_quality(),
        recording(archive()),
        QualityPreference::Source,
    );
    assert!(matches!(
        c.state(),
        PlaybackState::Ready { rewind: None, .. }
    ));
}

#[test]
fn a_rewind_starts_twenty_seconds_behind_the_uptime_and_never_before_the_start() {
    assert_eq!(REWIND_BEHIND, Seconds::new(20));
    assert_eq!(
        RewindPolicy::start(Seconds::new(3_600)),
        Seconds::new(3_580)
    );
    assert_eq!(RewindPolicy::start(Seconds::new(5)), Seconds::ZERO);
    assert_eq!(RewindPolicy::start(Seconds::new(20)), Seconds::ZERO);
}

#[test]
fn rewinding_loads_the_recording_where_the_stream_was_while_live_plays_on() {
    let c = loading();
    assert_eq!(
        rewind_of(&c),
        Some(&Rewind::Loading {
            video: archive(),
            start: Seconds::new(START)
        })
    );
    assert_eq!(opened(&c), Some(open("1080p60", 0, true)));
}

#[test]
fn rewinding_is_refused_unless_the_page_is_ready_and_offers_it() {
    let unavailable = live_page_with(Ok(live_details(None, Some(Timestamp::from_unix(STARTED)))));
    let mut resolving = live_page();
    resolving.begin(live("a"));
    for (mut c, target) in [
        (live_page(), live("b")),
        (resolving, live("a")),
        (loading(), live("a")),
        (rewound(), live("a")),
        (returning(), live("a")),
        (unavailable, live("a")),
    ] {
        let before = c.state().clone();
        assert_eq!(c.rewind(&target, an_hour_in()), None);
        assert_eq!(c.state(), &before);
    }
}

#[test]
fn a_failed_rewind_can_be_tried_again() {
    let mut c = loading();
    assert_eq!(
        apply_rewind(&mut c, Err(RewindFailure::Other("network".to_owned()))),
        Some(RewindOutcome::Failed("network".to_owned()))
    );
    assert_eq!(
        rewind_of(&c),
        Some(&Rewind::Failed {
            video: archive(),
            message: "network".to_owned()
        })
    );
    assert_eq!(opened(&c), Some(open("1080p60", 0, true)));
    assert_eq!(c.rewind(&live("a"), an_hour_in()), Some(archive()));
    assert!(matches!(rewind_of(&c), Some(Rewind::Loading { .. })));
}

#[test]
fn a_rewound_page_plays_the_recording_from_the_start_with_its_details() {
    let t0 = Instant::now();
    let mut c = loading();
    c.arm_sleep(15, t0);
    c.follow(&live("a"), Seconds::new(42), Seconds::new(100));
    assert_eq!(at_of(&c), Some(Seconds::new(42)));
    assert_eq!(
        apply_rewind(&mut c, Ok(vod_ladder())),
        Some(RewindOutcome::Rewound)
    );
    assert_eq!(
        rewind_of(&c),
        Some(&Rewind::Rewound {
            video: archive(),
            start: Seconds::new(START)
        })
    );
    assert_eq!(opened(&c), Some(vod_open("1080p60", START)));
    assert_eq!(details(&c), Some(&chaptered()));
    assert_eq!(c.chapter_start(1), Some(Seconds::new(3_600)));
    assert_eq!(at_of(&c), None);
    assert!(c.sleep_timer().is_some());
    assert!(matches!(c.state(), PlaybackState::Ready { target, .. } if *target == live("a")));
}

#[test]
fn a_rewind_keeps_the_quality_the_recording_has_else_takes_the_preferred() {
    let mut c = loading();
    c.select("720p60");
    c.apply_rewind(
        &live("a"),
        &archive(),
        Ok(vod_ladder()),
        no_details(),
        QualityPreference::Source,
    );
    assert_eq!(selected(&c), Some("720p60"));

    let mut c = loading();
    c.select("720p60");
    c.apply_rewind(
        &live("a"),
        &archive(),
        Ok(vod_playback(vec![
            vod_variant("chunked", 1080, QualityKind::Source),
            vod_variant("480p30", 480, QualityKind::Transcode),
        ])),
        no_details(),
        QualityPreference::P480,
    );
    assert_eq!(selected(&c), Some("480p30"));
}

#[test]
fn a_recording_without_qualities_fails_the_rewind_and_live_plays_on() {
    let mut c = loading();
    assert_eq!(
        apply_rewind(&mut c, Ok(vod_playback(vec![]))),
        Some(RewindOutcome::Failed("no playable qualities".to_owned()))
    );
    assert!(matches!(rewind_of(&c), Some(Rewind::Failed { .. })));
    assert_eq!(opened(&c), Some(open("1080p60", 0, true)));
}

#[test]
fn a_rewound_page_saves_no_resume_point() {
    let store = FakeResumeStore::default();
    let mut resume = ResumeController::new(Box::new(store.clone()));
    let c = rewound();
    assert_eq!(record(&c, &mut resume, &live("a"), 600, 3_600), None);
    assert_eq!(
        record(&c, &mut resume, &PlaybackTarget::Vod(archive()), 600, 3_600),
        None
    );
    assert!(store.points().is_empty());
    // Control: a VOD page does save.
    let c = ready(PlaybackTarget::Vod(vod()));
    assert_eq!(
        record(&c, &mut resume, &PlaybackTarget::Vod(vod()), 600, 3_600),
        Some(ResumeDecision::Save)
    );
    assert_eq!(store.points().len(), 1);
}

#[test]
fn a_refused_recording_turns_the_rewind_off_for_good_while_live_plays_on() {
    let mut c = loading();
    assert_eq!(
        apply_rewind(&mut c, Err(RewindFailure::Forbidden)),
        Some(RewindOutcome::Refused(NoRewind::Forbidden))
    );
    assert_eq!(
        rewind_of(&c),
        Some(&Rewind::Unavailable(NoRewind::Forbidden))
    );
    assert_eq!(opened(&c), Some(open("1080p60", 0, true)));
    assert!(!c.back_to_live(&live("a")));
    assert_eq!(c.rewind(&live("a"), an_hour_in()), None);

    // Reopening the page remembers it.
    c.begin(live("a"));
    c.apply(
        live("a"),
        Ok(live_playback()),
        recording(archive()),
        QualityPreference::Source,
    );
    assert_eq!(
        rewind_of(&c),
        Some(&Rewind::Unavailable(NoRewind::Forbidden))
    );

    // So does going back to live from another recording.
    let other = video_id("2002");
    c.begin(live("a"));
    c.apply(
        live("a"),
        Ok(live_playback()),
        recording(other.clone()),
        QualityPreference::Source,
    );
    assert_eq!(c.rewind(&live("a"), an_hour_in()), Some(other.clone()));
    c.apply_rewind(
        &live("a"),
        &other,
        Ok(vod_ladder()),
        no_details(),
        QualityPreference::Source,
    );
    assert!(c.back_to_live(&live("a")));
    assert_eq!(
        c.apply_return(
            &live("a"),
            &other,
            Ok(live_playback()),
            recording(archive()),
            QualityPreference::Source,
        ),
        Some(ReturnOutcome::Live)
    );
    assert_eq!(
        rewind_of(&c),
        Some(&Rewind::Unavailable(NoRewind::Forbidden))
    );
}

#[test]
fn a_recording_resolved_while_stopped_away_opens_at_the_rewind_start_not_the_live_position() {
    let mut c = loading();
    assert_eq!(
        c.background(false, true, Seconds::new(600)),
        Some(BackgroundStep::Stop)
    );
    apply_rewind(&mut c, Ok(vod_ladder()));
    assert_eq!(opened(&c), None);
    assert_eq!(
        foreground(&mut c, 0, &no_resume()),
        Some(vod_open("1080p60", START))
    );
}

#[test]
fn a_late_rewind_result_after_leaving_a_retry_or_for_another_video_is_ignored() {
    let mut c = loading();
    c.leave(&live("a"));
    assert_eq!(apply_rewind(&mut c, Ok(vod_ladder())), None);
    assert_eq!(c.state(), &PlaybackState::Idle);

    let mut c = loading();
    c.begin(live("a"));
    assert_eq!(apply_rewind(&mut c, Ok(vod_ladder())), None);
    assert_eq!(c.state(), &PlaybackState::Resolving { target: live("a") });

    let mut c = loading();
    let before = c.state().clone();
    assert_eq!(
        c.apply_rewind(
            &live("a"),
            &video_id("2002"),
            Ok(vod_ladder()),
            no_details(),
            QualityPreference::Source,
        ),
        None
    );
    assert_eq!(
        c.apply_rewind(
            &live("b"),
            &archive(),
            Ok(vod_ladder()),
            no_details(),
            QualityPreference::Source,
        ),
        None
    );
    assert_eq!(c.state(), &before);
}

#[test]
fn going_back_to_live_keeps_the_recording_playing_while_live_resolves() {
    let c = returning();
    assert_eq!(
        rewind_of(&c),
        Some(&Rewind::Returning {
            video: archive(),
            start: Seconds::new(START)
        })
    );
    assert_eq!(opened(&c), Some(vod_open("1080p60", START)));
}

#[test]
fn only_a_rewound_page_goes_back_to_live() {
    let mut stale = rewound();
    stale.begin(live("b"));
    for (mut c, target) in [
        (live_page(), live("a")),
        (loading(), live("a")),
        (returning(), live("a")),
        (rewound(), live("b")),
        (stale, live("a")),
    ] {
        let before = c.state().clone();
        assert!(!c.back_to_live(&target));
        assert_eq!(c.state(), &before);
    }
}

#[test]
fn back_at_live_the_page_plays_at_the_edge_and_offers_the_fresh_recording() {
    let t0 = Instant::now();
    let mut c = returning();
    c.arm_sleep(15, t0);
    c.follow(&live("a"), Seconds::new(4_000), Seconds::new(7_200));
    let restarted = video_id("2002");
    assert_eq!(
        apply_return(&mut c, Ok(live_playback()), recording(restarted.clone())),
        Some(ReturnOutcome::Live)
    );
    assert_eq!(opened(&c), Some(open("1080p60", 0, true)));
    assert_eq!(rewind_of(&c), Some(&Rewind::Available { video: restarted }));
    assert_eq!(
        details(&c).map(|d| d.title.as_deref()),
        Some(Some("live title"))
    );
    assert_eq!(at_of(&c), None);
    assert!(c.sleep_timer().is_some());
}

#[test]
fn back_at_live_the_page_keeps_the_quality_live_has_else_takes_the_preferred() {
    let mut c = returning();
    c.select("720p60");
    apply_return(&mut c, Ok(live_playback()), recording(archive()));
    assert_eq!(selected(&c), Some("720p60"));

    let mut c = loading();
    c.apply_rewind(
        &live("a"),
        &archive(),
        Ok(vod_playback(vec![vod_variant(
            "chunked",
            1080,
            QualityKind::Source,
        )])),
        no_details(),
        QualityPreference::Source,
    );
    c.back_to_live(&live("a"));
    assert_eq!(selected(&c), Some("chunked"));
    c.apply_return(
        &live("a"),
        &archive(),
        Ok(live_playback()),
        recording(archive()),
        QualityPreference::P720,
    );
    assert_eq!(selected(&c), Some("720p60"));
}

#[test]
fn a_failed_return_keeps_the_recording_playing_and_can_be_tried_again() {
    let t0 = Instant::now();
    let mut c = returning();
    c.arm_sleep(15, t0);
    assert_eq!(
        apply_return(
            &mut c,
            Err("This channel is offline.".to_owned()),
            recording(archive())
        ),
        Some(ReturnOutcome::Failed("This channel is offline.".to_owned()))
    );
    assert_eq!(
        rewind_of(&c),
        Some(&Rewind::Rewound {
            video: archive(),
            start: Seconds::new(START)
        })
    );
    assert_eq!(opened(&c), Some(vod_open("1080p60", START)));
    assert_eq!(details(&c), Some(&chaptered()));
    assert!(c.sleep_timer().is_some());
    assert!(c.back_to_live(&live("a")));

    assert_eq!(
        apply_return(&mut c, Ok(playback(vec![])), recording(archive())),
        Some(ReturnOutcome::Failed("no playable qualities".to_owned()))
    );
    assert!(matches!(rewind_of(&c), Some(Rewind::Rewound { .. })));
    assert_eq!(opened(&c), Some(vod_open("1080p60", START)));
}

#[test]
fn a_late_return_result_after_leaving_a_retry_the_sleep_timer_or_for_another_video_is_ignored() {
    let mut c = returning();
    c.leave(&live("a"));
    assert_eq!(
        apply_return(&mut c, Ok(live_playback()), recording(archive())),
        None
    );
    assert_eq!(c.state(), &PlaybackState::Idle);

    let mut c = returning();
    c.begin(live("a"));
    assert_eq!(
        apply_return(&mut c, Ok(live_playback()), recording(archive())),
        None
    );
    assert_eq!(c.state(), &PlaybackState::Resolving { target: live("a") });

    let t0 = Instant::now();
    let mut c = returning();
    c.arm_sleep(15, t0);
    assert_eq!(
        c.sleep_tick(t0 + Duration::from_secs(16 * 60)),
        Some(SleepStep::Expired)
    );
    assert_eq!(
        apply_return(&mut c, Ok(live_playback()), recording(archive())),
        None
    );
    assert_eq!(c.state(), &PlaybackState::Slept { target: live("a") });

    let mut c = returning();
    let before = c.state().clone();
    assert_eq!(
        c.apply_return(
            &live("a"),
            &video_id("2002"),
            Ok(live_playback()),
            recording(archive()),
            QualityPreference::Source,
        ),
        None
    );
    assert_eq!(
        c.apply_return(
            &live("b"),
            &archive(),
            Ok(live_playback()),
            recording(archive()),
            QualityPreference::Source,
        ),
        None
    );
    assert_eq!(c.state(), &before);
}

#[test]
fn back_at_live_without_details_plays_and_offers_no_rewind() {
    let mut c = returning();
    assert_eq!(
        apply_return(&mut c, Ok(live_playback()), Err("timed out".to_owned())),
        Some(ReturnOutcome::Live)
    );
    assert_eq!(opened(&c), Some(open("1080p60", 0, true)));
    assert_eq!(
        rewind_of(&c),
        Some(&Rewind::Unavailable(NoRewind::NoDetails))
    );
    assert_eq!(details(&c), Some(&PlaybackDetails::default()));
}

#[test]
fn a_rewound_page_carries_on_mid_recording_in_the_background_and_back() {
    for mut c in [rewound(), returning()] {
        assert_eq!(
            c.background(true, true, Seconds::new(4_000)),
            Some(BackgroundStep::Open(vod_open("audio_only", 4_000)))
        );
        assert_eq!(
            foreground(&mut c, 4_100, &no_resume()),
            Some(vod_open("1080p60", 4_100))
        );
        assert_eq!(
            c.switch_quality("720p60", Seconds::new(4_200)),
            Some(vod_open("720p60", 4_200))
        );
    }
}

#[test]
fn the_page_streams_the_recording_while_rewound_or_returning_and_live_otherwise() {
    let recording_stream = Some(PlaybackTarget::Vod(archive()));
    assert_eq!(rewound().streaming(&live("a")), recording_stream);
    assert_eq!(returning().streaming(&live("a")), recording_stream);
    assert_eq!(live_page().streaming(&live("a")), Some(live("a")));
    assert_eq!(loading().streaming(&live("a")), Some(live("a")));
    assert_eq!(rewound().streaming(&live("b")), None);
    let mut c = returning();
    apply_return(&mut c, Ok(live_playback()), recording(archive()));
    assert_eq!(c.streaming(&live("a")), Some(live("a")));
}

#[test]
fn chat_replay_follows_the_recording_by_what_the_page_streams() {
    let c = rewound();
    let mut replay = ChatReplay::new(archive());
    assert_eq!(
        replay.position(&live("a"), Seconds::new(START), Seconds::new(7_200)),
        Default::default()
    );
    assert!(replay.waiting());
    let Some(stream) = c.streaming(&live("a")) else {
        unreachable!("the page is ready");
    };
    assert!(
        replay
            .position(&stream, Seconds::new(START), Seconds::new(7_200))
            .fetch
            .is_some()
    );
}

// Refreshing what the page shows, the player left as it plays.

fn fresh_live(viewers: u32) -> Result<PlaybackDetails, String> {
    Ok(PlaybackDetails {
        title: Some("fresh title".to_owned()),
        live: Some(LiveDetails {
            viewers,
            started_at: Some(Timestamp::from_unix(STARTED)),
            archive: Some(archive()),
        }),
        ..PlaybackDetails::default()
    })
}

#[test]
fn refreshed_details_replace_the_shown_ones_and_leave_the_player_alone() {
    let mut c = live_page();
    let before = opened(&c);
    assert!(c.refresh_details(&live("a"), &live("a"), fresh_live(99)));
    assert_eq!(details(&c), fresh_live(99).ok().as_ref());
    assert_eq!(opened(&c), before);
    assert_eq!(selected(&c), Some("1080p60"));
}

#[test]
fn refreshed_details_that_failed_keep_the_shown_ones_and_the_rewind() {
    let mut c = live_page();
    assert!(!c.refresh_details(&live("a"), &live("a"), Err("offline".to_owned())));
    assert_eq!(details(&c), recording(archive()).ok().as_ref());
    assert_eq!(rewind_of(&c), Some(&Rewind::Available { video: archive() }));
}

#[test]
fn a_refresh_offers_the_rewind_the_fresh_details_call_for() {
    let mut c = live_page_with(Ok(live_details(None, Some(Timestamp::from_unix(STARTED)))));
    assert_eq!(
        rewind_of(&c),
        Some(&Rewind::Unavailable(NoRewind::NoRecording))
    );
    assert!(c.refresh_details(&live("a"), &live("a"), fresh_live(10)));
    assert_eq!(rewind_of(&c), Some(&Rewind::Available { video: archive() }));
}

#[test]
fn a_refresh_keeps_a_rewind_in_progress() {
    let mut c = loading();
    assert!(c.refresh_details(&live("a"), &live("a"), fresh_live(10)));
    assert!(matches!(rewind_of(&c), Some(Rewind::Loading { .. })));
}

#[test]
fn a_rewound_page_refreshes_its_recording_s_details_only() {
    let recording_stream = PlaybackTarget::Vod(archive());
    let mut c = rewound();
    // Asked for while live, answered after the rewind: stale.
    assert!(!c.refresh_details(&live("a"), &live("a"), fresh_live(10)));
    assert_eq!(details(&c), Some(&chaptered()));
    assert!(c.refresh_details(&live("a"), &recording_stream, Ok(gta())));
    assert_eq!(details(&c), Some(&gta()));
    assert!(matches!(rewind_of(&c), Some(Rewind::Rewound { .. })));
}

#[test]
fn a_refresh_for_a_page_left_or_replaced_changes_nothing() {
    let mut c = live_page();
    assert!(!c.refresh_details(&live("b"), &live("b"), fresh_live(10)));
    assert!(c.leave(&live("a")));
    assert!(!c.refresh_details(&live("a"), &live("a"), fresh_live(10)));
    assert_eq!(c.state(), &PlaybackState::Idle);
}
