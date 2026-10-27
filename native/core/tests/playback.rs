use jitter_core::fakes::FakeResumeStore;
use jitter_core::models::{
    ChannelLogin, Open, Playback, QualityKind, Resolution, ResumePoint, Seconds, Timestamp,
    Variant, VideoId,
};
use jitter_core::playback::{PlaybackController, PlaybackState, PlaybackTarget};
use jitter_core::resume::ResumeController;

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

fn selected(c: &PlaybackController) -> Option<&str> {
    c.selected_quality().map(|q| q.name.as_str())
}

#[test]
fn result_for_another_target_is_ignored() {
    let mut c = PlaybackController::new();
    c.begin(live("a"));
    c.begin(live("b"));
    c.apply(
        live("a"),
        Ok(playback(vec![variant("720p", 720, QualityKind::Transcode)])),
    );
    assert_eq!(c.state(), &PlaybackState::Resolving { target: live("b") });
    c.apply(live("a"), Err("late".to_owned()));
    assert_eq!(c.state(), &PlaybackState::Resolving { target: live("b") });
}

#[test]
fn result_while_idle_is_ignored() {
    let mut c = PlaybackController::new();
    c.apply(live("a"), Err("x".to_owned()));
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
    c.apply(live("a"), Err("offline".to_owned()));
    assert_eq!(
        c.state(),
        &PlaybackState::Failed {
            message: "offline".to_owned()
        }
    );
    c.begin(live("a"));
    c.apply(live("a"), Ok(playback(vec![])));
    assert!(matches!(c.state(), PlaybackState::Failed { .. }));
}

fn vod() -> VideoId {
    let Ok(id) = VideoId::parse("123") else {
        unreachable!("valid id");
    };
    id
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
