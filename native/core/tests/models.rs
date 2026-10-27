use jitter_core::models::{
    Category, CategoryId, CategoryIdError, Channel, ChannelId, ChannelIdError, ChannelLogin,
    ChannelLoginError, Playback, QualityKind, Resolution, Seconds, Stream, Timestamp, Variant,
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
