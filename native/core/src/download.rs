//! One VOD download: the file it writes and how far it got. The head fetches each URL in order and appends its bytes to the part file.
//!
//! After each URL's bytes are in the part file, the head overwrites a
//! checkpoint file beside it with `checkpoint()`. A later run hands that
//! text back to `resume`, which says how much of the part file to keep.
//!
//! An fMP4 VOD's timestamps start where the broadcast was, not at zero
//! (62 s in, say), and players then show a length that much too long and
//! seek past the end. So the head moves every track's timestamps back by
//! the same time as it writes them; `Timeline` records by how much, so a
//! resumed run moves its bytes as the first run did.

use crate::models::{ChannelLogin, SegmentList, VideoId};

/// A VOD quality being joined into one file. Core owns the name and the count;
/// the heads only fetch and write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Download {
    file_name: String,
    urls: Vec<String>,
    done: usize,
    bytes: u64,
    timeline: Timeline,
}

/// The checkpoint's first line: its format, so that another one is never
/// read as this one.
const CHECKPOINT_HEADER: &str = "jitter-download 2";

/// How the joined file's timestamps relate to the fetched ones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Timeline {
    /// fMP4 before its first media segment: where its timeline starts is
    /// not known yet.
    Undecided,
    /// Written as fetched: a TS VOD, whose offset players handle, or an
    /// fMP4 one the head could not read.
    Raw,
    /// Each listed track's timestamps moved back by its ticks, so the file
    /// starts at zero. A track not listed is written as fetched.
    Shifted(Vec<TrackShift>),
}

/// How far back one track's timestamps move, in that track's own ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackShift {
    pub track: u32,
    pub ticks: u64,
}

/// How the head opens the part file before it fetches `remaining()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartStart {
    /// Create it empty, truncating whatever is there.
    Fresh,
    /// Keep its first `keep` bytes and append after them. Anything past them
    /// is a segment cut off halfway.
    Resume { keep: u64 },
}

/// How far a download got: `done` of `total` URLs, `bytes` written so far.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownloadProgress {
    pub done: usize,
    pub total: usize,
    pub bytes: u64,
}

impl Download {
    /// `login` is None when the VOD's channel is unknown.
    pub fn new(
        login: Option<&ChannelLogin>,
        id: &VideoId,
        quality: &str,
        list: SegmentList,
    ) -> Self {
        let stem = match login {
            Some(login) => format!("{login}_{id}_{quality}"),
            None => format!("{id}_{quality}"),
        };
        let file_name = format!("{}.{}", safe_chars(&stem), list.extension());
        let timeline = match list.init {
            Some(_) => Timeline::Undecided,
            None => Timeline::Raw,
        };
        let urls = list.init.into_iter().chain(list.segments).collect();
        Self {
            file_name,
            urls,
            done: 0,
            bytes: 0,
            timeline,
        }
    }

    /// `<login>_<id>_<quality>.<ext>`, or `<id>_<quality>.<ext>` without a login.
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// Where the bytes go until the last part is written: `<file_name>.part`.
    /// A half-written file never carries the final name.
    pub fn part_name(&self) -> String {
        format!("{}.part", self.file_name)
    }

    /// Beside the part file until the final rename: `<file_name>.part.resume`.
    pub fn checkpoint_name(&self) -> String {
        format!("{}.part.resume", self.file_name)
    }

    /// The URLs still to fetch, in write order: the init segment first, if
    /// any. All of them unless `resume` picked up an earlier run.
    pub fn remaining(&self) -> &[String] {
        self.urls.get(self.done..).unwrap_or_default()
    }

    pub fn timeline(&self) -> &Timeline {
        &self.timeline
    }

    /// Settles an `Undecided` timeline, once the head has read the first
    /// media segment. A settled one stays as it is: bytes already written
    /// were moved by it.
    pub fn settle_timeline(&mut self, timeline: Timeline) {
        if self.timeline == Timeline::Undecided {
            self.timeline = timeline;
        }
    }

    /// What the checkpoint file holds once the part file has every byte
    /// `record`ed so far.
    pub fn checkpoint(&self) -> String {
        let last = self.done.checked_sub(1).and_then(|i| self.urls.get(i));
        format!(
            "{CHECKPOINT_HEADER}\ndone {}\nbytes {}\nlast {}\ntimeline {}\n",
            self.done,
            self.bytes,
            last.map_or("", String::as_str),
            write_timeline(&self.timeline)
        )
    }

    /// Picks up an earlier run of this download from its checkpoint text
    /// (`saved`) and its part file's length (`part_len`). It resumes only
    /// when the checkpoint is this format, names the same URL at the same
    /// place in this playlist, and counts no more bytes than the part file
    /// holds. Anything else starts over, because a wrong guess would join
    /// the wrong bytes into the file.
    ///
    /// An `Undecided` timeline starts over too: only the init is in, and
    /// the head reads its timescales from the init it fetches, so it has to
    /// fetch it again.
    pub fn resume(&mut self, saved: &str, part_len: u64) -> PartStart {
        let Some((done, bytes, timeline)) = self.parse_checkpoint(saved) else {
            return PartStart::Fresh;
        };
        if bytes > part_len || timeline == Timeline::Undecided {
            return PartStart::Fresh;
        }
        self.done = done;
        self.bytes = bytes;
        self.timeline = timeline;
        PartStart::Resume { keep: bytes }
    }

    /// `(done, bytes, timeline)` from a checkpoint of this playlist, or None.
    fn parse_checkpoint(&self, saved: &str) -> Option<(usize, u64, Timeline)> {
        let mut lines = saved.lines();
        if lines.next()? != CHECKPOINT_HEADER {
            return None;
        }
        let done: usize = lines.next()?.strip_prefix("done ")?.parse().ok()?;
        let bytes: u64 = lines.next()?.strip_prefix("bytes ")?.parse().ok()?;
        let last = lines.next()?.strip_prefix("last ")?;
        let timeline = read_timeline(lines.next()?.strip_prefix("timeline ")?)?;
        let same_url = self.urls.get(done.checked_sub(1)?)? == last;
        same_url.then_some((done, bytes, timeline))
    }

    /// One more URL's bytes were appended.
    pub fn record(&mut self, bytes: usize) {
        self.bytes = self
            .bytes
            .saturating_add(u64::try_from(bytes).unwrap_or(u64::MAX));
        self.done = self.done.saturating_add(1).min(self.urls.len());
    }

    pub fn progress(&self) -> DownloadProgress {
        DownloadProgress {
            done: self.done,
            total: self.urls.len(),
            bytes: self.bytes,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.done == self.urls.len()
    }
}

/// A timeline as the checkpoint writes it: `undecided`, `raw`, or
/// `shifted` and each track as `<track>:<ticks>`.
fn write_timeline(timeline: &Timeline) -> String {
    match timeline {
        Timeline::Undecided => "undecided".to_owned(),
        Timeline::Raw => "raw".to_owned(),
        Timeline::Shifted(tracks) => tracks.iter().fold("shifted".to_owned(), |line, shift| {
            format!("{line} {}:{}", shift.track, shift.ticks)
        }),
    }
}

fn read_timeline(line: &str) -> Option<Timeline> {
    let mut words = line.split(' ');
    match words.next()? {
        "undecided" => Some(Timeline::Undecided),
        "raw" => Some(Timeline::Raw),
        "shifted" => words
            .map(|word| {
                let (track, ticks) = word.split_once(':')?;
                Some(TrackShift {
                    track: track.parse().ok()?,
                    ticks: ticks.parse().ok()?,
                })
            })
            .collect::<Option<Vec<_>>>()
            .map(Timeline::Shifted),
        _ => None,
    }
}

/// Every character that is not ASCII alphanumeric, `_` or `-` becomes `_`, so a
/// quality such as `a b/c` cannot make a path.
fn safe_chars(stem: &str) -> String {
    stem.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
