use jitter_core::download::{Download, DownloadProgress};
use jitter_core::models::{ChannelLogin, SegmentList, VideoId};

fn login() -> ChannelLogin {
    let Ok(login) = ChannelLogin::parse("shroud") else {
        unreachable!("valid login");
    };
    login
}

fn vod() -> VideoId {
    let Ok(id) = VideoId::parse("2931745878") else {
        unreachable!("valid id");
    };
    id
}

fn ts(segments: &[&str]) -> SegmentList {
    SegmentList {
        init: None,
        segments: segments.iter().map(|s| (*s).to_owned()).collect(),
    }
}

fn fmp4(init: &str, segments: &[&str]) -> SegmentList {
    SegmentList {
        init: Some(init.to_owned()),
        ..ts(segments)
    }
}

#[test]
fn file_name_joins_login_id_quality_and_extension() {
    let download = Download::new(Some(&login()), &vod(), "1080p60", ts(&["s0"]));
    assert_eq!(download.file_name(), "shroud_2931745878_1080p60.ts");
    assert_eq!(download.part_name(), "shroud_2931745878_1080p60.ts.part");
}

#[test]
fn file_name_without_a_login_is_id_and_quality() {
    let download = Download::new(None, &vod(), "720p60", fmp4("i", &["s0"]));
    assert_eq!(download.file_name(), "2931745878_720p60.mp4");
}

#[test]
fn odd_characters_in_the_quality_become_underscores() {
    let download = Download::new(Some(&login()), &vod(), "a b/c.d", ts(&["s0"]));
    assert_eq!(download.file_name(), "shroud_2931745878_a_b_c_d.ts");
}

#[test]
fn urls_put_the_init_segment_first() {
    let download = Download::new(None, &vod(), "480p", fmp4("i", &["s0", "s1"]));
    assert_eq!(download.urls(), ["i", "s0", "s1"]);
}

#[test]
fn record_counts_parts_and_bytes_until_finished() {
    let mut download = Download::new(None, &vod(), "480p", ts(&["s0", "s1"]));
    assert_eq!(
        download.progress(),
        DownloadProgress {
            done: 0,
            total: 2,
            bytes: 0
        }
    );
    assert!(!download.is_finished());

    download.record(10);
    download.record(5);
    assert_eq!(
        download.progress(),
        DownloadProgress {
            done: 2,
            total: 2,
            bytes: 15
        }
    );
    assert!(download.is_finished());

    download.record(1);
    assert_eq!(download.progress().done, 2);
}
