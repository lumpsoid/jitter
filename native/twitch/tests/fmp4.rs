//! fMP4 rebase tests against hand-built boxes shaped like Twitch's: an init
//! with a 48 kHz audio track and a 1 MHz video track, segments of `moof` +
//! `mdat` pairs. No network.

use jitter_core::download::{Download, Timeline, TrackShift};
use jitter_core::models::{SegmentList, VideoId};
use jitter_twitch::fmp4;
use jitter_twitch::part_file::Rebase;

const AUDIO: u32 = 1;
const VIDEO: u32 = 2;
/// Where Twitch's clock stood at the VOD's first fragment: 62.016667 s of
/// audio, 62.033 s of video.
const AUDIO_START: u64 = 2_976_800;
const VIDEO_START: u64 = 62_033_000;

fn bx(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let size = u32::try_from(payload.len() + 8).unwrap_or(u32::MAX);
    [&size.to_be_bytes()[..], kind, payload].concat()
}

/// A full box: version, zero flags, then `body`.
fn full(kind: &[u8; 4], version: u8, body: &[u8]) -> Vec<u8> {
    bx(kind, &[&[version, 0, 0, 0][..], body].concat())
}

/// A track's `trak`, its `tkhd` and `mdhd` at `version`.
fn trak(track: u32, timescale: u32, version: u8) -> Vec<u8> {
    // Creation and modification times: 4 bytes each, 8 at version 1.
    let times = vec![0u8; if version == 1 { 16 } else { 8 }];
    let tkhd = full(
        b"tkhd",
        version,
        &[&times[..], &track.to_be_bytes(), &[0; 4]].concat(),
    );
    let mdhd = full(
        b"mdhd",
        version,
        &[&times[..], &timescale.to_be_bytes(), &[0; 4]].concat(),
    );
    bx(b"trak", &[tkhd, bx(b"mdia", &mdhd)].concat())
}

fn init(video_version: u8) -> Vec<u8> {
    let moov = [
        full(b"mvhd", 0, &[0; 96]),
        trak(AUDIO, 48_000, 0),
        trak(VIDEO, 1_000_000, video_version),
        bx(b"mvex", &[]),
    ]
    .concat();
    [bx(b"ftyp", b"iso6mp41"), bx(b"moov", &moov)].concat()
}

/// One `moof` + `mdat`: a `traf` per `(track, version, start)`.
fn fragment(times: &[(u32, u8, u64)]) -> Vec<u8> {
    let trafs: Vec<u8> = times
        .iter()
        .flat_map(|&(track, version, start)| {
            let tfdt = match version {
                1 => full(b"tfdt", 1, &start.to_be_bytes()),
                _ => full(
                    b"tfdt",
                    0,
                    &u32::try_from(start).unwrap_or(u32::MAX).to_be_bytes(),
                ),
            };
            let tfhd = full(b"tfhd", 0, &track.to_be_bytes());
            bx(b"traf", &[tfhd, tfdt].concat())
        })
        .collect();
    let moof = bx(b"moof", &[full(b"mfhd", 0, &[0; 4]), trafs].concat());
    [moof, bx(b"mdat", b"media bytes")].concat()
}

/// A segment of two fragments, 0.1 s apart, starting at `audio` and `video`.
fn segment(audio: u64, video: u64) -> Vec<u8> {
    [
        bx(b"emsg", &[0; 16]),
        fragment(&[(AUDIO, 0, audio), (VIDEO, 0, video)]),
        fragment(&[(AUDIO, 0, audio + 4_800), (VIDEO, 0, video + 100_000)]),
    ]
    .concat()
}

/// Each fragment's `(track, start)`, read straight from the bytes.
fn starts(segment: &[u8]) -> Vec<(u32, u64)> {
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(i) = find(segment, b"tfhd", at) {
        let track = u32::from_be_bytes(read(segment, i + 8));
        let tfdt = find(segment, b"tfdt", i).unwrap_or(0);
        let start = if segment.get(tfdt + 4) == Some(&1) {
            u64::from_be_bytes(read(segment, tfdt + 8))
        } else {
            u64::from(u32::from_be_bytes(read(segment, tfdt + 8)))
        };
        found.push((track, start));
        at = tfdt;
    }
    found
}

/// Where the box type `kind` next appears after `from`.
fn find(buf: &[u8], kind: &[u8; 4], from: usize) -> Option<usize> {
    buf.windows(4)
        .skip(from + 1)
        .position(|w| w == kind)
        .map(|i| i + from + 1)
}

fn read<const N: usize>(buf: &[u8], at: usize) -> [u8; N] {
    buf.get(at..at + N)
        .and_then(|b| b.try_into().ok())
        .unwrap_or([0; N])
}

fn shifts() -> Vec<TrackShift> {
    vec![
        TrackShift {
            track: AUDIO,
            ticks: AUDIO_START,
        },
        // The audio's 62.016667 s in video ticks, rounded down.
        TrackShift {
            track: VIDEO,
            ticks: 62_016_666,
        },
    ]
}

#[test]
fn timescales_come_from_each_track_of_the_init() {
    for video_version in [0, 1] {
        let Some(timescales) = fmp4::timescales(&init(video_version)) else {
            unreachable!("expected timescales at version {video_version}");
        };
        let first = segment(AUDIO_START, VIDEO_START);
        assert_eq!(fmp4::shifts(&timescales, &first), Some(shifts()));
    }
}

#[test]
fn every_track_moves_back_by_the_earliest_start() {
    let timescales = fmp4::timescales(&init(0));
    let first = segment(AUDIO_START, VIDEO_START);
    let shifts = timescales.and_then(|t| fmp4::shifts(&t, &first));
    assert_eq!(shifts, Some(self::shifts()));
}

#[test]
fn a_shift_rewrites_each_fragment_time_and_no_other_byte() {
    let original = segment(AUDIO_START, VIDEO_START);
    let mut moved = original.clone();

    assert_eq!(fmp4::shift(&mut moved, &shifts()), 0);
    assert_eq!(moved.len(), original.len());
    // The video keeps its 16.334 ms lead over the audio.
    assert_eq!(
        starts(&moved),
        [
            (AUDIO, 0),
            (VIDEO, 16_334),
            (AUDIO, 4_800),
            (VIDEO, 116_334)
        ]
    );
    let changed = moved.iter().zip(&original).filter(|(a, b)| a != b).count();
    assert!(changed <= 4 * 4, "{changed} bytes changed");
    assert!(moved.ends_with(b"media bytes"));
}

#[test]
fn a_wide_fragment_time_stays_wide() {
    let later = 10_992_033_000;
    let original = fragment(&[(AUDIO, 0, 527_618_080), (VIDEO, 1, later)]);
    let mut moved = original.clone();

    fmp4::shift(&mut moved, &shifts());
    assert_eq!(moved.len(), original.len());
    assert_eq!(
        starts(&moved),
        [
            (AUDIO, 527_618_080 - AUDIO_START),
            (VIDEO, later - 62_016_666)
        ]
    );
}

#[test]
fn a_time_before_its_shift_becomes_zero_and_is_counted() {
    let mut early = fragment(&[(AUDIO, 0, 10), (VIDEO, 0, VIDEO_START)]);
    assert_eq!(fmp4::shift(&mut early, &shifts()), 1);
    assert_eq!(starts(&early), [(AUDIO, 0), (VIDEO, 16_334)]);
}

#[test]
fn an_init_or_segment_of_another_shape_reads_as_none() {
    assert_eq!(fmp4::timescales(b"not an mp4"), None);
    assert_eq!(fmp4::timescales(&bx(b"ftyp", b"iso6")), None);
    let Some(timescales) = fmp4::timescales(&init(0)) else {
        unreachable!("expected timescales");
    };
    // The video track has no fragment in it.
    let audio_only = fragment(&[(AUDIO, 0, AUDIO_START)]);
    assert_eq!(fmp4::shifts(&timescales, &audio_only), None);
    assert_eq!(fmp4::shifts(&timescales, b"\0\0\0\x08mdat"), None);
}

fn download(init: bool) -> Download {
    let Ok(video) = VideoId::parse("42") else {
        unreachable!("valid id");
    };
    let list = SegmentList {
        init: init.then(|| "init-0.mp4".to_owned()),
        segments: vec!["0.mp4".to_owned(), "1.mp4".to_owned()],
    };
    Download::new(None, &video, "1080p60", list)
}

#[test]
fn a_rebase_keeps_the_init_and_starts_the_first_segment_at_zero() {
    let mut download = download(true);
    let mut rebase = Rebase::new();

    let mut init = init(0);
    rebase.prepare(&mut download, &mut init);
    assert_eq!(init, self::init(0));
    assert_eq!(download.timeline(), &Timeline::Undecided);
    download.record(init.len());

    let mut first = segment(AUDIO_START, VIDEO_START);
    rebase.prepare(&mut download, &mut first);
    assert_eq!(download.timeline(), &Timeline::Shifted(shifts()));
    assert_eq!(starts(&first).first(), Some(&(AUDIO, 0)));
    download.record(first.len());

    let mut second = segment(AUDIO_START + 96_000, VIDEO_START + 2_000_000);
    rebase.prepare(&mut download, &mut second);
    assert_eq!(starts(&second).first(), Some(&(AUDIO, 96_000)));
}

#[test]
fn a_resumed_rebase_moves_the_rest_by_the_saved_shifts() {
    let mut earlier = download(true);
    earlier.record(10);
    earlier.settle_timeline(Timeline::Shifted(shifts()));
    earlier.record(20);
    let mut download = download(true);
    download.resume(&earlier.checkpoint(), 30);

    let mut second = segment(AUDIO_START + 96_000, VIDEO_START + 2_000_000);
    Rebase::new().prepare(&mut download, &mut second);
    assert_eq!(starts(&second).first(), Some(&(AUDIO, 96_000)));
}

#[test]
fn an_unreadable_init_writes_the_file_as_fetched() {
    let mut download = download(true);
    let mut rebase = Rebase::new();

    let mut init = b"not an mp4".to_vec();
    rebase.prepare(&mut download, &mut init);
    assert_eq!(download.timeline(), &Timeline::Raw);

    let original = segment(AUDIO_START, VIDEO_START);
    let mut first = original.clone();
    rebase.prepare(&mut download, &mut first);
    assert_eq!(first, original);
}

#[test]
fn a_ts_download_is_written_as_fetched() {
    let mut download = download(false);
    let original = b"\x47 transport stream".to_vec();
    let mut bytes = original.clone();
    Rebase::new().prepare(&mut download, &mut bytes);
    assert_eq!(bytes, original);
}
