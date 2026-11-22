//! The part file a VOD download writes into, shared by the heads. Core
//! decides from the checkpoint where the download picks up; this opens the
//! file there, and readies each fetched URL's bytes before they go in.

use std::fs::File;
use std::io::{self, Seek, SeekFrom};
use std::path::Path;

use jitter_core::download::{Download, PartStart, Timeline};

use crate::fmp4::{self, Timescales};

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

/// Readies each fetched URL's bytes for the part file: moves an fMP4
/// download's timestamps as its `Timeline` says, and settles a timeline
/// still `Undecided` from the init and the first media segment. One per
/// run of a download, handed every URL's bytes in order before `record`.
#[derive(Debug, Default)]
pub struct Rebase {
    /// The init's timescales, between the init and the first media segment.
    timescales: Option<Timescales>,
}

impl Rebase {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn prepare(&mut self, download: &mut Download, bytes: &mut [u8]) {
        if download.timeline() == &Timeline::Undecided {
            self.settle(download, bytes);
        }
        if let Timeline::Shifted(shifts) = download.timeline() {
            let clamped = fmp4::shift(bytes, shifts);
            if clamped > 0 {
                log::warn!("{clamped} fragment times were before the file's start; set to zero");
            }
        }
    }

    /// An `Undecided` download is fetched from its init: core starts an
    /// undecided one over rather than resume it. So the first bytes here
    /// are the init, and the next are the first media segment. Anything
    /// unreadable leaves the whole file as fetched, decided before any
    /// media byte is written.
    fn settle(&mut self, download: &mut Download, bytes: &[u8]) {
        let timeline = match self.timescales.take() {
            None => match fmp4::timescales(bytes) {
                Some(timescales) => {
                    self.timescales = Some(timescales);
                    return;
                }
                None => {
                    log::warn!("unreadable fMP4 init; writing its timestamps as fetched");
                    Timeline::Raw
                }
            },
            Some(timescales) => match fmp4::shifts(&timescales, bytes) {
                Some(shifts) => Timeline::Shifted(shifts),
                None => {
                    log::warn!(
                        "no fragment times in the first fMP4 segment; writing them as fetched"
                    );
                    Timeline::Raw
                }
            },
        };
        download.settle_timeline(timeline);
    }
}
