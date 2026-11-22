use jitter_core::download::{Download, DownloadProgress, PartStart, Timeline, TrackShift};
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
fn remaining_puts_the_init_segment_first() {
    let download = Download::new(None, &vod(), "480p", fmp4("i", &["s0", "s1"]));
    assert_eq!(download.remaining(), ["i", "s0", "s1"]);
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

/// A download of three segments.
fn fresh() -> Download {
    Download::new(None, &vod(), "480p", ts(&["s0", "s1", "s2"]))
}

/// `fresh()`'s checkpoint after `done` segments of 10 bytes each.
fn interrupted(done: usize) -> String {
    let mut download = fresh();
    for _ in 0..done {
        download.record(10);
    }
    download.checkpoint()
}

#[test]
fn checkpoint_names_sit_beside_the_part_file() {
    assert_eq!(fresh().checkpoint_name(), "2931745878_480p.ts.part.resume");
}

#[test]
fn resume_keeps_the_checkpointed_bytes_and_skips_their_urls() {
    let saved = interrupted(2);
    let mut download = fresh();

    // 25 bytes on disk: the third segment was cut off halfway.
    assert_eq!(download.resume(&saved, 25), PartStart::Resume { keep: 20 });
    assert_eq!(download.remaining(), ["s2"]);
    assert_eq!(
        download.progress(),
        DownloadProgress {
            done: 2,
            total: 3,
            bytes: 20
        }
    );
}

#[test]
fn resume_of_a_complete_part_leaves_nothing_to_fetch() {
    let saved = interrupted(3);
    let mut download = fresh();

    assert_eq!(download.resume(&saved, 30), PartStart::Resume { keep: 30 });
    assert!(download.remaining().is_empty());
    assert!(download.is_finished());
}

#[test]
fn resume_starts_over_when_the_part_file_is_shorter_than_its_checkpoint() {
    let saved = interrupted(2);
    let mut download = fresh();

    assert_eq!(download.resume(&saved, 19), PartStart::Fresh);
    assert_eq!(download.remaining().len(), 3);
}

#[test]
fn resume_starts_over_for_another_playlist() {
    let saved = interrupted(2);
    let mut other = Download::new(None, &vod(), "480p", ts(&["t0", "t1", "t2"]));
    assert_eq!(other.resume(&saved, 20), PartStart::Fresh);

    let mut shorter = Download::new(None, &vod(), "480p", ts(&["s0"]));
    assert_eq!(shorter.resume(&saved, 20), PartStart::Fresh);
}

#[test]
fn resume_starts_over_for_a_checkpoint_it_cannot_read() {
    let nothing_done = interrupted(0);
    for saved in [
        "",
        "garbage",
        // Format 1, from before the timeline line.
        "jitter-download 1\ndone 1\nbytes 10\nlast s0\n",
        "jitter-download 2\ndone 1\nbytes 10\nlast s0\ntimeline sideways\n",
        "jitter-download 2\ndone 1\nbytes 10\nlast s0\ntimeline shifted 1:x\n",
        &nothing_done,
    ] {
        assert_eq!(fresh().resume(saved, 100), PartStart::Fresh, "{saved:?}");
    }
}

/// An fMP4 download: the init, then two segments.
fn fresh_fmp4() -> Download {
    Download::new(None, &vod(), "480p", fmp4("init", &["s0", "s1"]))
}

fn shifted() -> Timeline {
    Timeline::Shifted(vec![
        TrackShift {
            track: 1,
            ticks: 2_976_800,
        },
        TrackShift {
            track: 2,
            ticks: 62_016_666,
        },
    ])
}

#[test]
fn only_an_fmp4_timeline_waits_to_be_decided() {
    assert_eq!(fresh().timeline(), &Timeline::Raw);
    assert_eq!(fresh_fmp4().timeline(), &Timeline::Undecided);
}

#[test]
fn a_settled_timeline_stays_as_it_is() {
    let mut download = fresh_fmp4();
    download.settle_timeline(shifted());
    download.settle_timeline(Timeline::Raw);
    assert_eq!(download.timeline(), &shifted());

    let mut ts = fresh();
    ts.settle_timeline(shifted());
    assert_eq!(ts.timeline(), &Timeline::Raw);
}

#[test]
fn resume_moves_the_rest_as_the_first_run_did() {
    let mut earlier = fresh_fmp4();
    earlier.record(5);
    earlier.settle_timeline(shifted());
    earlier.record(10);

    let mut download = fresh_fmp4();
    assert_eq!(
        download.resume(&earlier.checkpoint(), 15),
        PartStart::Resume { keep: 15 }
    );
    assert_eq!(download.timeline(), &shifted());
    assert_eq!(download.remaining(), ["s1"]);
}

#[test]
fn resume_keeps_a_raw_fmp4_timeline_raw() {
    let mut earlier = fresh_fmp4();
    earlier.record(5);
    earlier.settle_timeline(Timeline::Raw);
    earlier.record(10);

    let mut download = fresh_fmp4();
    assert_eq!(
        download.resume(&earlier.checkpoint(), 15),
        PartStart::Resume { keep: 15 }
    );
    assert_eq!(download.timeline(), &Timeline::Raw);
}

#[test]
fn resume_after_only_the_init_starts_over() {
    let mut earlier = fresh_fmp4();
    earlier.record(5);

    let mut download = fresh_fmp4();
    assert_eq!(download.resume(&earlier.checkpoint(), 5), PartStart::Fresh);
    assert_eq!(download.remaining(), ["init", "s0", "s1"]);
    assert_eq!(download.timeline(), &Timeline::Undecided);
}
