//! The part file a VOD download writes into, shared by the heads. Core
//! decides from the checkpoint where the download picks up; this opens the
//! file there.

use std::fs::File;
use std::io::{self, Seek, SeekFrom};
use std::path::Path;

use jitter_core::download::{Download, PartStart};

/// Opens `part` where `download` picks up: after an earlier run's bytes when
/// `checkpoint` still fits them, else empty. A resume moves `download` past
/// the segments already in the file.
pub fn open(part: &Path, checkpoint: &Path, download: &mut Download) -> io::Result<File> {
    let start = match saved_run(part, checkpoint) {
        Some((saved, part_len)) => download.resume(&saved, part_len),
        None => PartStart::Fresh,
    };
    match start {
        PartStart::Fresh => File::create(part),
        PartStart::Resume { keep } => {
            let mut file = File::options().write(true).open(part)?;
            file.set_len(keep)?;
            file.seek(SeekFrom::End(0))?;
            Ok(file)
        }
    }
}

/// The checkpoint's text and the part file's length, when both are there.
fn saved_run(part: &Path, checkpoint: &Path) -> Option<(String, u64)> {
    let saved = std::fs::read_to_string(checkpoint).ok()?;
    let part_len = std::fs::metadata(part).ok()?.len();
    Some((saved, part_len))
}
