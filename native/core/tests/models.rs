use std::time::{Duration, Instant};

use jitter_core::models::{
    Category, CategoryId, CategoryIdError, Channel, ChannelId, ChannelIdError, ChannelLogin,
    ChannelLoginError, ClockError, FadeLevel, LiveDetails, Playback, QualityKind, Resolution,
    SLEEP_FADE, SLEEP_FADE_STEP, Seconds, SleepStep, SleepTimer, Stream, Timestamp, Variant,
    VideoId, VideoIdError,
};

fn login(s: &str) -> ChannelLogin {
    ChannelLogin::parse(s).unwrap_or_else(|e| unreachable!("{e:?}"))
}

#[test]
fn login_accepts_and_lowercases() {
    assert_eq!(login("xqc").as_str(), "xqc");
    assert_eq!(login("some_user_1").as_str(), "some_user_1");
    assert_eq!(login("XQC").as_str(), "xqc");
    assert!(ChannelLogin::parse(&"a".repeat(25)).is_ok());
}

#[test]
fn login_rejects_invalid() {
    assert_eq!(ChannelLogin::parse(""), Err(ChannelLoginError::Empty));
    assert_eq!(
        ChannelLogin::parse("a b"),
        Err(ChannelLoginError::InvalidChar(' '))
    );
    assert_eq!(
        ChannelLogin::parse("#xqc"),
        Err(ChannelLoginError::InvalidChar('#'))
    );
    assert_eq!(
        ChannelLogin::parse(&"a".repeat(26)),
        Err(ChannelLoginError::TooLong)
    );
}

#[test]
fn ids_reject_non_numeric_and_empty() {
    assert_eq!(ChannelId::parse("v123"), Err(ChannelIdError::NotNumeric));
    assert_eq!(ChannelId::parse(""), Err(ChannelIdError::Empty));
    assert_eq!(VideoId::parse("v123"), Err(VideoIdError::NotNumeric));
    assert_eq!(VideoId::parse(""), Err(VideoIdError::Empty));
    assert_eq!(
        VideoId::parse("123").map(|v| v.to_string()),
        Ok("123".into())
    );
}

fn variant(
    name: &str,
    kind: QualityKind,
    res: Option<(u32, u32)>,
    fps: Option<u32>,
    bw: u64,
) -> Variant {
    Variant {
        name: name.into(),
        url: format!("https://x/{name}"),
        resolution: res.map(|(width, height)| Resolution { width, height }),
        frame_rate: fps,
        bandwidth: bw,
        kind,
    }
}

fn names(p: &Playback) -> Vec<&str> {
    p.qualities.iter().map(|q| q.name.as_str()).collect()
}

fn shuffled() -> Vec<Variant> {
    use QualityKind::{AudioOnly, Source, Transcode};
    vec![
        variant("480p30", Transcode, Some((852, 480)), Some(30), 1_000),
        variant("audio_only", AudioOnly, None, None, 100),
        variant("720p60", Transcode, Some((1280, 720)), Some(60), 3_000),
        variant("160p30", Transcode, Some((284, 160)), Some(30), 300),
        variant("1080p60", Source, Some((1920, 1080)), Some(60), 6_000),
    ]
}

#[test]
fn playback_orders_source_transcodes_audio() {
    let p = Playback::from_variants("m".into(), shuffled());
    assert_eq!(
        names(&p),
        ["1080p60", "720p60", "480p30", "160p30", "audio_only"]
    );
    assert_eq!(
        p.default_quality().map(|q| q.name.as_str()),
        Some("1080p60")
    );
}

#[test]
fn playback_without_source_puts_highest_resolution_first() {
    let v: Vec<_> = shuffled()
        .into_iter()
        .filter(|v| v.kind != QualityKind::Source)
        .collect();
    let p = Playback::from_variants("m".into(), v);
    assert_eq!(names(&p), ["720p60", "480p30", "160p30", "audio_only"]);
    assert_eq!(p.default_quality().map(|q| q.name.as_str()), Some("720p60"));
}

#[test]
fn playback_ties_order_by_bandwidth_descending() {
    use QualityKind::Transcode;
    let v = vec![
        variant("low", Transcode, Some((1280, 720)), Some(30), 1_000),
        variant("high", Transcode, Some((1280, 720)), Some(30), 2_000),
    ];
    let p = Playback::from_variants("m".into(), v);
    assert_eq!(names(&p), ["high", "low"]);
}

#[test]
fn seconds_subtraction_saturates() {
    assert_eq!(Seconds::new(5) - Seconds::new(9), Seconds::ZERO);
    assert_eq!(Seconds::new(9) - Seconds::new(5), Seconds::new(4));
    assert_eq!(
        Seconds::new(u32::MAX) + Seconds::new(1),
        Seconds::new(u32::MAX)
    );
}

#[test]
fn stream_thumbnail_fills_template() {
    let stream = Stream {
        id: "1".into(),
        channel: Channel {
            id: ChannelId::parse("7").unwrap_or_else(|e| unreachable!("{e:?}")),
            login: login("xqc"),
            display_name: "xQc".into(),
            avatar_url: String::new(),
        },
        title: "t".into(),
        game: None,
        viewers: 1,
        started_at: None,
        thumbnail_url: "https://t/live-{width}x{height}.jpg".into(),
    };
    assert_eq!(stream.thumbnail(320, 180), "https://t/live-320x180.jpg");
}

#[test]
fn stream_thumbnail_without_template_is_unchanged() {
    let stream = Stream {
        id: "1".into(),
        channel: Channel {
            id: ChannelId::parse("7").unwrap_or_else(|e| unreachable!("{e:?}")),
            login: login("xqc"),
            display_name: "xQc".into(),
            avatar_url: String::new(),
        },
        title: "t".into(),
        game: None,
        viewers: 1,
        started_at: None,
        thumbnail_url: "https://t/live-440x248.jpg".into(),
    };
    assert_eq!(stream.thumbnail(320, 180), "https://t/live-440x248.jpg");
}

fn started(at: Option<i64>) -> Stream {
    Stream {
        id: "1".into(),
        channel: Channel {
            id: ChannelId::parse("7").unwrap_or_else(|e| unreachable!("{e:?}")),
            login: login("xqc"),
            display_name: "xQc".into(),
            avatar_url: String::new(),
        },
        title: "t".into(),
        game: None,
        viewers: 1,
        started_at: at.map(Timestamp::from_unix),
        thumbnail_url: String::new(),
    }
}

#[test]
fn uptime_is_the_time_since_the_start() {
    let now = Timestamp::from_unix(1_000_000);
    let stream = started(Some(1_000_000 - 11_520));
    assert_eq!(stream.uptime(now), Some(Seconds::new(11_520)));
    assert_eq!(started(Some(1_000_000)).uptime(now), Some(Seconds::ZERO));
}

#[test]
fn uptime_is_unknown_without_a_start() {
    assert_eq!(started(None).uptime(Timestamp::from_unix(100)), None);
}

#[test]
fn a_start_after_now_reads_as_just_started() {
    assert_eq!(
        started(Some(200)).uptime(Timestamp::from_unix(100)),
        Some(Seconds::ZERO)
    );
}

#[test]
fn an_uptime_beyond_u32_saturates() {
    assert_eq!(
        started(Some(i64::MIN)).uptime(Timestamp::from_unix(i64::MAX)),
        Some(Seconds::new(u32::MAX))
    );
}

#[test]
fn a_live_streams_uptime_is_the_time_since_it_went_live() {
    let live = |at: Option<i64>| LiveDetails {
        viewers: 1_234,
        started_at: at.map(Timestamp::from_unix),
        archive: None,
    };
    let now = Timestamp::from_unix(1_000_000);
    assert_eq!(
        live(Some(1_000_000 - 8_100)).uptime(now),
        Some(Seconds::new(8_100))
    );
    assert_eq!(live(Some(1_000_100)).uptime(now), Some(Seconds::ZERO));
    assert_eq!(live(None).uptime(now), None);
}

#[test]
fn category_ids_reject_non_numeric_and_empty() {
    assert_eq!(
        CategoryId::parse("33214").map(|id| id.to_string()),
        Ok("33214".to_owned())
    );
    assert_eq!(
        CategoryId::parse("elden-ring"),
        Err(CategoryIdError::NotNumeric)
    );
    assert_eq!(CategoryId::parse(""), Err(CategoryIdError::Empty));
}

fn category(box_art_url: &str) -> Category {
    Category {
        id: CategoryId::parse("21779").unwrap_or_else(|e| unreachable!("{e:?}")),
        name: "League of Legends".into(),
        box_art_url: box_art_url.into(),
        viewers: 1,
    }
}

#[test]
fn box_art_fills_template() {
    let c = category("https://b/21779-{width}x{height}.jpg");
    assert_eq!(c.box_art(285, 380), "https://b/21779-285x380.jpg");
}

#[test]
fn box_art_swaps_a_fixed_size() {
    let c = category("https://static-cdn.jtvnw.net/ttv-boxart/21779-40x56.jpg");
    assert_eq!(
        c.box_art(285, 380),
        "https://static-cdn.jtvnw.net/ttv-boxart/21779-285x380.jpg"
    );
}

#[test]
fn box_art_without_a_size_is_unchanged() {
    let c = category("https://static-cdn.jtvnw.net/ttv-boxart/21779_IGDB.jpg");
    assert_eq!(
        c.box_art(285, 380),
        "https://static-cdn.jtvnw.net/ttv-boxart/21779_IGDB.jpg"
    );
    let c = category("https://b/elden-ring.jpg");
    assert_eq!(c.box_art(285, 380), "https://b/elden-ring.jpg");
}

// Sleep timer: the clock is a plain value; any instant serves as the origin.

fn secs(s: u64) -> Duration {
    Duration::from_secs(s)
}

fn armed(minutes: u64) -> (SleepTimer, Instant) {
    let t0 = Instant::now();
    let Some(timer) = SleepTimer::arm(secs(minutes * 60), t0) else {
        unreachable!("a positive duration arms");
    };
    (timer, t0)
}

#[test]
fn a_zero_sleep_timer_does_not_arm() {
    assert_eq!(SleepTimer::arm(Duration::ZERO, Instant::now()), None);
}

#[test]
fn a_sleep_timer_counts_down_to_its_deadline() {
    let (timer, t0) = armed(15);
    assert_eq!(timer.minutes_left(), 15);
    assert_eq!(timer.remaining(t0 + secs(60)), secs(14 * 60));
    assert!(!timer.expired(t0 + secs(15 * 60 - 1)));
    assert!(timer.expired(t0 + secs(15 * 60)));
    assert_eq!(timer.remaining(t0 + secs(16 * 60)), Duration::ZERO);
}

#[test]
fn the_volume_fades_linearly_over_the_last_stretch() {
    let (timer, t0) = armed(1);
    let end = t0 + secs(60);
    assert_eq!(timer.fade_level(t0), FadeLevel::FULL);
    assert_eq!(timer.fade_level(end - SLEEP_FADE), FadeLevel::FULL);
    assert_eq!(timer.fade_level(end - SLEEP_FADE / 2), FadeLevel::new(0.5));
    assert_eq!(timer.fade_level(end), FadeLevel::new(0.0));
}

#[test]
fn the_time_left_rounds_up_and_wakes_at_each_minute() {
    let (mut timer, t0) = armed(15);
    assert_eq!(timer.wake(), t0 + secs(60));
    assert_eq!(
        timer.tick(t0 + secs(30)),
        SleepStep::Counting {
            relabeled: false,
            fade: None
        }
    );
    assert_eq!(timer.minutes_left(), 15);
    assert_eq!(timer.wake(), t0 + secs(60));
    assert_eq!(
        timer.tick(t0 + secs(60)),
        SleepStep::Counting {
            relabeled: true,
            fade: None
        }
    );
    assert_eq!(timer.minutes_left(), 14);
    assert_eq!(timer.wake(), t0 + secs(120));
}

#[test]
fn the_last_minute_wakes_at_the_fade_then_every_step_until_the_deadline() {
    let (mut timer, t0) = armed(2);
    let end = t0 + secs(120);
    timer.tick(t0 + secs(60));
    assert_eq!(timer.minutes_left(), 1);
    assert_eq!(timer.wake(), end - SLEEP_FADE);
    assert_eq!(
        timer.tick(end - SLEEP_FADE),
        SleepStep::Counting {
            relabeled: false,
            fade: Some(FadeLevel::FULL)
        }
    );
    assert_eq!(timer.wake(), end - SLEEP_FADE + SLEEP_FADE_STEP);
    let near = end - SLEEP_FADE_STEP / 2;
    assert!(matches!(
        timer.tick(near),
        SleepStep::Counting { fade: Some(level), .. } if level < FadeLevel::new(0.05)
    ));
    assert_eq!(timer.wake(), end);
    assert_eq!(timer.tick(end), SleepStep::Expired);
}

#[test]
fn a_fade_level_is_clamped_to_silence_and_full() {
    assert_eq!(FadeLevel::new(1.5), FadeLevel::FULL);
    assert_eq!(FadeLevel::new(-0.1).get(), 0.0);
    assert_eq!(FadeLevel::new(f64::NAN).get(), 0.0);
    assert_eq!(FadeLevel::new(0.25).get(), 0.25);
}

fn emote_with(still_url: Option<&str>) -> jitter_core::models::Emote {
    jitter_core::models::Emote {
        id: "1".to_owned(),
        name: "Kappa".to_owned(),
        url: "https://e/animated".to_owned(),
        still_url: still_url.map(str::to_owned),
        zero_width: false,
    }
}

#[test]
fn an_emote_shows_its_still_only_with_animation_off() {
    let emote = emote_with(Some("https://e/still"));
    assert_eq!(emote.image_url(true), "https://e/animated");
    assert_eq!(emote.image_url(false), "https://e/still");
}

#[test]
fn an_emote_without_a_still_shows_its_url_either_way() {
    let emote = emote_with(None);
    assert_eq!(emote.image_url(true), "https://e/animated");
    assert_eq!(emote.image_url(false), "https://e/animated");
}

fn clock(input: &str) -> Result<u32, ClockError> {
    Seconds::parse_clock(input).map(Seconds::get)
}

#[test]
fn a_clock_reads_hours_minutes_and_seconds() {
    assert_eq!(clock("1:23:45"), Ok(5025));
    assert_eq!(clock("23:45"), Ok(1425));
    assert_eq!(clock("45"), Ok(45));
    assert_eq!(clock(" 0:05 "), Ok(5));
    assert_eq!(clock("90:00"), Ok(5400));
}

#[test]
fn units_read_in_any_subset_largest_first() {
    assert_eq!(clock("1h23m45s"), Ok(5025));
    assert_eq!(clock("1h23m"), Ok(4980));
    assert_eq!(clock("1h 23m"), Ok(4980));
    assert_eq!(clock("1H23M"), Ok(4980));
    assert_eq!(clock("90m"), Ok(5400));
    assert_eq!(clock("45s"), Ok(45));
    assert_eq!(clock("2h5s"), Ok(7205));
}

#[test]
fn a_typo_is_not_a_time() {
    assert_eq!(clock(""), Err(ClockError::Empty));
    assert_eq!(clock("   "), Err(ClockError::Empty));
    for typo in [
        "1:75",
        "1:2:3:4",
        ":30",
        "1::30",
        "1h75m",
        "23m1h",
        "1h1h",
        "1x",
        "h",
        "1 h",
        "-5",
        "1.5h",
        "1:30s",
        "99999999999",
    ] {
        assert_eq!(clock(typo), Err(ClockError::Malformed), "{typo}");
    }
}

#[test]
fn a_position_goes_no_further_than_the_end() {
    let duration = Seconds::new(3600);
    assert_eq!(
        Seconds::parse_position("1:00:00", duration),
        Ok(Seconds::new(3600))
    );
    assert_eq!(
        Seconds::parse_position("1:00:01", duration),
        Err(ClockError::PastEnd { duration })
    );
    assert_eq!(
        Seconds::parse_position("9h", Seconds::ZERO),
        Ok(Seconds::new(32400))
    );
}
