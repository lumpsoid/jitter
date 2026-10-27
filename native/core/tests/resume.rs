use jitter_core::fakes::FakeResumeStore;
use jitter_core::models::{ResumePoint, Seconds, Timestamp, VideoId};
use jitter_core::resume::ResumeController;
use jitter_core::resume_policy::{ResumeDecision, ResumePolicy};

fn s(value: u32) -> Seconds {
    Seconds::new(value)
}

fn vod() -> VideoId {
    let Ok(id) = VideoId::parse("123") else {
        unreachable!("valid id");
    };
    id
}

fn point(position: u32, duration: u32) -> ResumePoint {
    ResumePoint {
        video_id: vod(),
        position: s(position),
        duration: s(duration),
        saved_at: Timestamp::from_unix(0),
    }
}

fn controller(store: &FakeResumeStore) -> ResumeController {
    ResumeController::new(Box::new(store.clone()))
}

fn now() -> Timestamp {
    Timestamp::from_unix(1_000)
}

#[test]
fn ten_seconds_into_an_hour_starts_at_zero() {
    assert_eq!(ResumePolicy::start_position(s(10), s(3600)), s(0));
}

#[test]
fn half_an_hour_into_an_hour_starts_there() {
    assert_eq!(ResumePolicy::start_position(s(1800), s(3600)), s(1800));
}

#[test]
fn thirty_seconds_before_the_end_is_finished_clears_and_starts_at_zero() {
    assert!(ResumePolicy::is_finished(s(3570), s(3600)));
    assert_eq!(ResumePolicy::start_position(s(3570), s(3600)), s(0));

    let store = FakeResumeStore::with_points(vec![point(1800, 3600)]);
    let mut resume = controller(&store);
    assert_eq!(
        resume.record(vod(), s(3570), s(3600), now()),
        Ok(ResumeDecision::Clear)
    );
    assert!(store.points().is_empty());
    assert_eq!(resume.start_position(&vod()), Ok(s(0)));
}

#[test]
fn a_short_vod_uses_the_percentage_tail_not_the_sixty_second_one() {
    // 95 % of 50 s is 47.5 s; 50 s - 60 s would be negative.
    assert!(!ResumePolicy::is_finished(s(40), s(50)));
    assert_eq!(ResumePolicy::start_position(s(40), s(50)), s(40));
    assert!(!ResumePolicy::is_finished(s(47), s(50)));
    assert!(ResumePolicy::is_finished(s(48), s(50)));
}

#[test]
fn the_tail_closer_to_the_end_wins_on_a_long_vod() {
    // 95 % of 3600 s is 3420 s; 3600 s - 60 s is 3540 s, closer to the end.
    assert!(!ResumePolicy::is_finished(s(3500), s(3600)));
    assert!(ResumePolicy::is_finished(s(3540), s(3600)));
}

#[test]
fn a_position_past_the_duration_is_finished() {
    assert!(ResumePolicy::is_finished(s(4000), s(3600)));
    assert_eq!(ResumePolicy::start_position(s(4000), s(3600)), s(0));
    assert_eq!(
        ResumePolicy::decide(s(4000), s(3600), Some(s(1800))),
        ResumeDecision::Clear
    );
}

#[test]
fn records_one_second_apart_skip_and_two_seconds_apart_save() {
    let store = FakeResumeStore::default();
    let mut resume = controller(&store);
    assert_eq!(
        resume.record(vod(), s(100), s(3600), now()),
        Ok(ResumeDecision::Save)
    );
    assert_eq!(
        resume.record(vod(), s(101), s(3600), now()),
        Ok(ResumeDecision::Skip)
    );
    assert_eq!(
        resume.record(vod(), s(102), s(3600), now()),
        Ok(ResumeDecision::Save)
    );
    assert_eq!(
        store.points(),
        [ResumePoint {
            saved_at: now(),
            ..point(102, 3600)
        }]
    );
}

#[test]
fn a_record_600_seconds_before_the_last_save_saves() {
    let store = FakeResumeStore::default();
    let mut resume = controller(&store);
    assert_eq!(
        resume.record(vod(), s(1800), s(3600), now()),
        Ok(ResumeDecision::Save)
    );
    assert_eq!(
        resume.record(vod(), s(1200), s(3600), now()),
        Ok(ResumeDecision::Save)
    );
    assert_eq!(store.points()[0].position, s(1200));
}

#[test]
fn start_position_uses_the_stored_duration_and_is_zero_with_no_point() {
    let empty = FakeResumeStore::default();
    assert_eq!(controller(&empty).start_position(&vod()), Ok(s(0)));

    // 1800 s is finished only against a 1810 s duration: the stored one decides.
    let short = FakeResumeStore::with_points(vec![point(1800, 1810)]);
    assert_eq!(controller(&short).start_position(&vod()), Ok(s(0)));
    let long = FakeResumeStore::with_points(vec![point(1800, 3600)]);
    assert_eq!(controller(&long).start_position(&vod()), Ok(s(1800)));
}

#[test]
fn record_seeds_the_last_saved_position_from_the_store() {
    let store = FakeResumeStore::with_points(vec![point(1800, 3600)]);
    let mut resume = controller(&store);
    assert_eq!(
        resume.record(vod(), s(1801), s(3600), now()),
        Ok(ResumeDecision::Skip)
    );
    assert_eq!(store.points(), [point(1800, 3600)]);
}

#[test]
fn a_finished_vod_with_nothing_stored_is_skipped() {
    let store = FakeResumeStore::default();
    let mut resume = controller(&store);
    assert_eq!(
        resume.record(vod(), s(3590), s(3600), now()),
        Ok(ResumeDecision::Skip)
    );
}

#[test]
fn a_failed_save_returns_err_and_the_next_record_tries_again() {
    let store = FakeResumeStore::default().failing_writes();
    let mut resume = controller(&store);
    assert!(resume.record(vod(), s(100), s(3600), now()).is_err());
    // The failed save did not count as the last save, so this is not throttled.
    assert!(resume.record(vod(), s(101), s(3600), now()).is_err());
    assert!(store.points().is_empty());
}

#[test]
fn progress_is_where_playback_would_resume() {
    let half = FakeResumeStore::with_points(vec![point(1800, 3600)]);
    assert_eq!(controller(&half).progress(&vod()), Ok(Some(0.5)));
}

#[test]
fn no_progress_when_playback_would_start_from_the_beginning() {
    let empty = FakeResumeStore::default();
    assert_eq!(controller(&empty).progress(&vod()), Ok(None));
    let barely_started = FakeResumeStore::with_points(vec![point(10, 3600)]);
    assert_eq!(controller(&barely_started).progress(&vod()), Ok(None));
    let finished = FakeResumeStore::with_points(vec![point(3590, 3600)]);
    assert_eq!(controller(&finished).progress(&vod()), Ok(None));
    let empty_duration = FakeResumeStore::with_points(vec![point(100, 0)]);
    assert_eq!(controller(&empty_duration).progress(&vod()), Ok(None));
}

#[test]
fn a_failed_read_returns_err() {
    let store = FakeResumeStore::with_points(vec![point(1800, 3600)]).failing_loads();
    let mut resume = controller(&store);
    assert!(resume.start_position(&vod()).is_err());
    assert!(resume.progress(&vod()).is_err());
    assert!(resume.record(vod(), s(1900), s(3600), now()).is_err());
}
