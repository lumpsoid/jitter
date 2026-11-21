use std::io::Write;
use std::path::PathBuf;

use jitter_core::download::Download;
use jitter_core::models::{SegmentList, VideoId};
use jitter_twitch::part_file;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn download() -> Result<Download, Box<dyn std::error::Error>> {
    let id = VideoId::parse("42").map_err(|_| "valid id")?;
    let list = SegmentList {
        init: None,
        segments: vec!["s0".into(), "s1".into(), "s2".into()],
    };
    Ok(Download::new(None, &id, "480p", list))
}

/// The part file and checkpoint of `download` in a fresh directory.
fn paths(
    download: &Download,
) -> Result<(tempfile::TempDir, PathBuf, PathBuf), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let part = dir.path().join(download.part_name());
    let checkpoint = dir.path().join(download.checkpoint_name());
    Ok((dir, part, checkpoint))
}

#[test]
fn without_a_checkpoint_it_starts_the_part_file_over() -> TestResult {
    let mut download = download()?;
    let (_dir, part, checkpoint) = paths(&download)?;
    std::fs::write(&part, b"stale bytes")?;

    part_file::open(&part, &checkpoint, &mut download)?;
    assert_eq!(std::fs::metadata(&part)?.len(), 0);
    assert_eq!(download.remaining(), ["s0", "s1", "s2"]);
    Ok(())
}

#[test]
fn a_fitting_checkpoint_cuts_a_half_segment_and_appends_after_it() -> TestResult {
    let mut earlier = download()?;
    let (_dir, part, checkpoint) = paths(&earlier)?;
    earlier.record(4);
    std::fs::write(&checkpoint, earlier.checkpoint())?;
    // The second segment was cut off after two bytes.
    std::fs::write(&part, b"aaaabb")?;

    let mut download = download()?;
    let mut file = part_file::open(&part, &checkpoint, &mut download)?;
    assert_eq!(download.remaining(), ["s1", "s2"]);
    file.write_all(b"BBBB")?;
    drop(file);
    assert_eq!(std::fs::read(&part)?, b"aaaaBBBB");
    Ok(())
}
