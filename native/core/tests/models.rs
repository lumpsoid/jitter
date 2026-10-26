use jitter_core::models::{
    Channel, ChannelId, ChannelIdError, ChannelLogin, ChannelLoginError, Playback, QualityKind,
    Resolution, Seconds, Stream, Variant, VideoId, VideoIdError,
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
        game_name: None,
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
        game_name: None,
        viewers: 1,
        started_at: None,
        thumbnail_url: "https://t/live-440x248.jpg".into(),
    };
    assert_eq!(stream.thumbnail(320, 180), "https://t/live-440x248.jpg");
}
