use std::time::{Duration, Instant};

use jitter_core::fakes::FakeResumeStore;
use jitter_core::models::{
    CategoryId, ChannelLogin, FadeLevel, GameRef, Open, Playback, PlaybackDetails, QualityKind,
    Resolution, ResumePoint, SLEEP_FADE, Seconds, SleepStep, Timestamp, Variant, VideoId,
};
use jitter_core::playback::{PlaybackController, PlaybackState, PlaybackTarget, TIMED_OUT};
use jitter_core::resume::ResumeController;
use jitter_core::resume_policy::ResumeDecision;

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
        no_details()
    ));
    assert_eq!(c.state(), &PlaybackState::Resolving { target: live("b") });
    assert!(!c.apply(live("a"), Err("late".to_owned()), no_details()));
    assert_eq!(c.state(), &PlaybackState::Resolving { target: live("b") });
}

#[test]
fn result_while_idle_is_ignored() {
    let mut c = PlaybackController::new();
    c.apply(live("a"), Err("x".to_owned()), no_details());
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
    assert!(!c.apply(live("a"), one_quality(), no_details()));
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
    assert!(c.apply(live("a"), one_quality(), no_details()));
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
    assert!(c.apply(live("a"), Err("offline".to_owned()), no_details()));
    assert_eq!(
        c.state(),
        &PlaybackState::Failed {
            message: "offline".to_owned()
        }
    );
    c.begin(live("a"));
    c.apply(live("a"), Ok(playback(vec![])), no_details());
    assert!(matches!(c.state(), PlaybackState::Failed { .. }));
}

fn gta() -> PlaybackDetails {
    let Ok(id) = CategoryId::parse("32982") else {
        unreachable!("valid id");
    };
    PlaybackDetails {
        channel: None,
        game: Some(GameRef {
            id,
            name: "Grand Theft Auto V".to_owned(),
        }),
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
    assert!(c.apply(live("a"), one_quality(), Ok(gta())));
    assert_eq!(details(&c), Some(&gta()));
}

#[test]
fn details_that_failed_to_load_still_play_without_them() {
    let mut c = PlaybackController::new();
    c.begin(live("a"));
    assert!(c.apply(live("a"), one_quality(), Err("timed out".to_owned())));
    assert_eq!(selected(&c), Some("720p"));
    assert_eq!(details(&c), Some(&PlaybackDetails::default()));
}

#[test]
fn details_do_not_rescue_a_failed_resolve() {
    let mut c = PlaybackController::new();
    c.begin(live("a"));
    assert!(c.apply(live("a"), Err("offline".to_owned()), Ok(gta())));
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
        game: None,
        streamed_at: Some(Timestamp::from_unix(1_792_875_046)),
        live: None,
    };
    let mut c = PlaybackController::new();
    c.begin(PlaybackTarget::Vod(vod()));
    assert!(c.apply(
        PlaybackTarget::Vod(vod()),
        one_quality(),
        Ok(streamed.clone())
    ));
    assert_eq!(details(&c), Some(&streamed));
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
    c.apply(live("a"), Err("offline".to_owned()), no_details());
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
    c.apply(live("a"), one_quality(), no_details());
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
