//! Media playlist tests against hand-written VOD fixtures shaped like Twitch's
//! DVR playlists. No network.

use jitter_core::models::SegmentList;
use jitter_twitch::TwitchError;
use jitter_twitch::hls::parse_media;

const TS: &[u8] = include_bytes!("fixtures/vod_media_ts.m3u8");
const FMP4: &[u8] = include_bytes!("fixtures/vod_media_fmp4.m3u8");
const PLAYLIST: &str =
    "https://d2vi6trrdongqn.cloudfront.net/abc_shroud_1_2/1080p60/index-dvr.m3u8";
const QUALITY: &str = "https://d2vi6trrdongqn.cloudfront.net/abc_shroud_1_2/1080p60/";

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

#[test]
fn ts_playlist_resolves_segments_against_the_playlist_url() {
    let list = ok(parse_media(TS, PLAYLIST));
    assert_eq!(
        list,
        SegmentList {
            init: None,
            segments: vec![
                format!("{QUALITY}0.ts"),
                format!("{QUALITY}1-muted.ts"),
                format!("{QUALITY}2-muted.ts"),
            ],
        }
    );
    assert_eq!(list.extension(), "ts");
}

#[test]
fn fmp4_playlist_has_its_init_segment() {
    let list = ok(parse_media(FMP4, PLAYLIST));
    assert_eq!(
        list.init.as_deref(),
        Some(format!("{QUALITY}init-0.mp4").as_str())
    );
    assert_eq!(
        list.segments,
        [format!("{QUALITY}0.mp4"), format!("{QUALITY}1.mp4")]
    );
    assert_eq!(list.extension(), "mp4");
}

#[test]
fn an_unmuted_fmp4_segment_is_left_as_listed() {
    let body = b"#EXTM3U\n#EXT-X-TARGETDURATION:10\n\
                 #EXT-X-MAP:URI=\"init-0.mp4\"\n\
                 #EXTINF:10.000,\n0-unmuted.mp4\n#EXTINF:10.000,\n1-muted.mp4\n\
                 #EXT-X-ENDLIST\n";
    assert_eq!(
        ok(parse_media(body, PLAYLIST)).segments,
        [
            format!("{QUALITY}0-unmuted.mp4"),
            format!("{QUALITY}1-muted.mp4")
        ]
    );
}

#[test]
fn a_playlist_without_segments_is_decode() {
    let body = b"#EXTM3U\n#EXT-X-TARGETDURATION:10\n#EXT-X-ENDLIST\n";
    assert!(matches!(
        err(parse_media(body, PLAYLIST)),
        TwitchError::Decode(_)
    ));
}

#[test]
fn two_different_init_segments_are_decode() {
    let body = b"#EXTM3U\n#EXT-X-TARGETDURATION:10\n\
                 #EXT-X-MAP:URI=\"init-0.mp4\"\n#EXTINF:10.000,\n0.mp4\n\
                 #EXT-X-MAP:URI=\"init-1.mp4\"\n#EXTINF:10.000,\n1.mp4\n\
                 #EXT-X-ENDLIST\n";
    assert!(matches!(
        err(parse_media(body, PLAYLIST)),
        TwitchError::Decode(_)
    ));
}
