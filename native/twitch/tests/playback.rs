//! Playback tests against the Phase 2 recorded fixtures. No network.
//!
//! Hand-written (Twitch never produced it): the token whose `value` has
//! `"forbidden":true`. Every other body is a recording.

use jitter_core::models::{ChannelLogin, QualityKind, Resolution, VideoId};
use jitter_core::playback::PlaybackTarget;
use jitter_twitch::TwitchError;
use jitter_twitch::gql::playback_access_token::{self, AccessToken, Subject};
use jitter_twitch::hls::parse_master;
use jitter_twitch::playback::{usher_error, usher_live_url, usher_vod_url};

const TOKEN_LIVE: &[u8] = include_bytes!("fixtures/gql_token_live.json");
const TOKEN_LIVE_OFFLINE: &[u8] = include_bytes!("fixtures/gql_token_live_offline_channel.json");
const TOKEN_VOD: &[u8] = include_bytes!("fixtures/gql_token_vod.json");
const TOKEN_UNKNOWN: &[u8] = include_bytes!("fixtures/gql_token_unknown_login.json");
const TOKEN_DELETED_VOD: &[u8] = include_bytes!("fixtures/gql_token_deleted_vod.json");
const LIVE_MASTER: &[u8] = include_bytes!("fixtures/usher_live_master.m3u8");
const VOD_MASTER: &[u8] = include_bytes!("fixtures/usher_vod_master.m3u8");

fn ok<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(e) => unreachable!("expected Ok, got {e:?}"),
    }
}

fn err<T: std::fmt::Debug, E>(result: Result<T, E>) -> E {
    match result {
        Err(e) => e,
        Ok(v) => unreachable!("expected Err, got {v:?}"),
    }
}

fn token() -> AccessToken {
    AccessToken {
        value: r#"{"a":"b c","n":1}"#.into(),
        signature: "abc123".into(),
    }
}

// --- URLs --------------------------------------------------------------------

#[test]
fn live_url_is_exact_and_percent_encoded() {
    let url = usher_live_url(&ok(ChannelLogin::parse("xqc")), &token(), 42);
    assert_eq!(
        url,
        "https://usher.ttvnw.net/api/v2/channel/hls/xqc.m3u8?allow_source=true\
         &allow_audio_only=true&fast_bread=true&include_unavailable=true&p=42\
         &platform=web&sig=abc123&supported_codecs=av1%2Ch265%2Ch264\
         &token=%7B%22a%22%3A%22b+c%22%2C%22n%22%3A1%7D"
    );
}

#[test]
fn vod_url_is_exact_and_has_no_fast_bread() {
    let url = usher_vod_url(&ok(VideoId::parse("2909822254")), &token(), 7);
    assert_eq!(
        url,
        "https://usher.ttvnw.net/vod/v2/2909822254.m3u8?allow_source=true\
         &allow_audio_only=true&include_unavailable=true&p=7&platform=web\
         &sig=abc123&supported_codecs=av1%2Ch265%2Ch264\
         &token=%7B%22a%22%3A%22b+c%22%2C%22n%22%3A1%7D"
    );
}

// --- tokens ------------------------------------------------------------------

#[test]
fn live_and_vod_tokens_parse() {
    let live = ok(playback_access_token::parse_live(TOKEN_LIVE));
    assert_eq!(live.signature, "acefa9b6c50c97a3b1f39430db910d381e7de540");
    assert!(live.value.contains("\"channel\":\"xqc\""));
    let vod = ok(playback_access_token::parse_vod(TOKEN_VOD));
    assert_eq!(vod.signature, "fc6de58cd5346baee8acb5bea47d70c21775765d");
    // An offline channel still gets a normal token; offline shows at usher.
    ok(playback_access_token::parse_live(TOKEN_LIVE_OFFLINE));
}

#[test]
fn null_tokens_are_not_found() {
    assert_eq!(
        err(playback_access_token::parse_live(TOKEN_UNKNOWN)),
        TwitchError::NotFound
    );
    assert_eq!(
        err(playback_access_token::parse_vod(TOKEN_DELETED_VOD)),
        TwitchError::NotFound
    );
}

#[test]
fn forbidden_token_is_playback_forbidden() {
    // Hand-written: "forbidden":true was never observed.
    let body = br#"{"data":{"streamPlaybackAccessToken":{"value":"{\"authorization\":{\"forbidden\":true,\"reason\":\"sub only\"}}","signature":"s"}}}"#;
    assert_eq!(
        err(playback_access_token::parse_live(body)),
        TwitchError::PlaybackForbidden
    );
    let body = br#"{"data":{"videoPlaybackAccessToken":{"value":"{\"authorization\":{\"forbidden\":true,\"reason\":\"\"}}","signature":"s"}}}"#;
    assert_eq!(
        err(playback_access_token::parse_vod(body)),
        TwitchError::PlaybackForbidden
    );
}

#[test]
fn token_value_without_authorization_is_a_decode_error() {
    let body = br#"{"data":{"streamPlaybackAccessToken":{"value":"{}","signature":"s"}}}"#;
    assert!(matches!(
        err(playback_access_token::parse_live(body)),
        TwitchError::Decode(_)
    ));
}

#[test]
fn token_variables_match_the_subject() {
    let login = ok(ChannelLogin::parse("xqc"));
    let v = playback_access_token::variables(Subject::Live(&login));
    assert_eq!(v["isLive"], true);
    assert_eq!(v["login"], "xqc");
    assert_eq!(v["isVod"], false);
    assert_eq!(v["vodID"], "");
    assert_eq!(v["playerType"], "site");
    assert_eq!(v["platform"], "web");
    let id = ok(VideoId::parse("42"));
    let v = playback_access_token::variables(Subject::Vod(&id));
    assert_eq!(v["isLive"], false);
    assert_eq!(v["isVod"], true);
    assert_eq!(v["vodID"], "42");
}

// --- usher status --------------------------------------------------------------

#[test]
fn usher_status_classification() {
    let live = PlaybackTarget::Live(ok(ChannelLogin::parse("xqc")));
    let vod = PlaybackTarget::Vod(ok(VideoId::parse("1")));
    assert_eq!(usher_error(404, &live), TwitchError::Offline);
    assert_eq!(usher_error(404, &vod), TwitchError::NotFound);
    assert_eq!(usher_error(403, &live), TwitchError::PlaybackForbidden);
    assert_eq!(usher_error(500, &vod), TwitchError::Http { status: 500 });
}

#[test]
fn recorded_usher_error_bodies_are_the_statuses_classified() {
    // The fixtures record `status: <n>` on their first line.
    let offline = include_str!("fixtures/usher_live_offline.txt");
    let bad_sig = include_str!("fixtures/usher_live_bad_sig.txt");
    let live = PlaybackTarget::Live(ok(ChannelLogin::parse("xqc")));
    assert!(offline.starts_with("status: 404"));
    assert_eq!(usher_error(404, &live), TwitchError::Offline);
    assert!(bad_sig.starts_with("status: 403"));
    assert_eq!(usher_error(403, &live), TwitchError::PlaybackForbidden);
}

// --- master playlists --------------------------------------------------------------

fn names_and_kinds(body: &[u8]) -> Vec<(String, QualityKind)> {
    ok(parse_master(body))
        .into_iter()
        .map(|v| (v.name, v.kind))
        .collect()
}

#[test]
fn live_master_parses_names_and_kinds() {
    let expected = [
        ("1080p60", QualityKind::Source),
        ("720p60", QualityKind::Transcode),
        ("480p30", QualityKind::Transcode),
        ("360p30", QualityKind::Transcode),
        ("160p30", QualityKind::Transcode),
        ("audio_only", QualityKind::AudioOnly),
    ];
    let got = names_and_kinds(LIVE_MASTER);
    assert_eq!(got.len(), expected.len());
    for ((name, kind), (want_name, want_kind)) in got.iter().zip(expected) {
        assert_eq!((name.as_str(), *kind), (want_name, want_kind));
    }
}

#[test]
fn vod_master_names_audio_only_by_stable_id_not_ivs_name() {
    let got = names_and_kinds(VOD_MASTER);
    assert_eq!(
        got.first(),
        Some(&("1080p60".to_owned(), QualityKind::Source))
    );
    // IVS-NAME is "Audio Only" here; STABLE-VARIANT-ID is the name.
    assert_eq!(
        got.last(),
        Some(&("audio_only".to_owned(), QualityKind::AudioOnly))
    );
    assert_eq!(got.len(), 6);
}

#[test]
fn variant_fields_come_from_the_stream_inf_line() {
    let variants = ok(parse_master(VOD_MASTER));
    let source = &variants[0];
    assert_eq!(source.bandwidth, 8_442_052);
    assert_eq!(
        source.resolution,
        Some(Resolution {
            width: 1920,
            height: 1080
        })
    );
    assert_eq!(source.frame_rate, Some(60));
    assert!(source.url.contains("/chunked/index-muted-"));
    let audio = &variants[5];
    assert_eq!(audio.resolution, None);
    assert_eq!(audio.frame_rate, None);
}

#[test]
fn master_becomes_playback_in_display_order() {
    let playback = jitter_core::models::Playback::from_variants(
        "https://usher.example/m".into(),
        ok(parse_master(LIVE_MASTER)),
    );
    let names: Vec<&str> = playback.qualities.iter().map(|q| q.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "1080p60",
            "720p60",
            "480p30",
            "360p30",
            "160p30",
            "audio_only"
        ]
    );
    assert_eq!(
        playback.default_quality().map(|q| q.name.as_str()),
        Some("1080p60")
    );
}

#[test]
fn garbage_is_not_a_master_playlist() {
    assert!(matches!(
        err(parse_master(b"not a playlist")),
        TwitchError::Decode(_)
    ));
}

#[test]
fn entries_without_a_stable_variant_id_are_dropped() {
    let body = b"#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=100,RESOLUTION=2x2\nhttp://x/a.m3u8\n\
                 #EXT-X-STREAM-INF:BANDWIDTH=200,STABLE-VARIANT-ID=\"b\"\nhttp://x/b.m3u8\n";
    let names = names_and_kinds(body);
    assert_eq!(names, [("b".to_owned(), QualityKind::Transcode)]);
}
