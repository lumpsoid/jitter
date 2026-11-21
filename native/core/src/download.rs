//! One VOD download: the file it writes and how far it got. The head fetches each URL in order and appends its bytes to the part file.
//!
//! After each URL's bytes are in the part file, the head overwrites a
//! checkpoint file beside it with `checkpoint()`. A later run hands that
//! text back to `resume`, which says how much of the part file to keep.

use crate::models::{ChannelLogin, SegmentList, VideoId};

/// A VOD quality being joined into one file. Core owns the name and the count;
/// the heads only fetch and write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Download {
    file_name: String,
    urls: Vec<String>,
    done: usize,
    bytes: u64,
}

/// The checkpoint's first line: its format, so that another one is never
/// read as this one.
const CHECKPOINT_HEADER: &str = "jitter-download 1";

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
        let urls = list.init.into_iter().chain(list.segments).collect();
        Self {
            file_name,
            urls,
            done: 0,
            bytes: 0,
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

    /// What the checkpoint file holds once the part file has every byte
    /// `record`ed so far.
    pub fn checkpoint(&self) -> String {
        let last = self.done.checked_sub(1).and_then(|i| self.urls.get(i));
        format!(
            "{CHECKPOINT_HEADER}\ndone {}\nbytes {}\nlast {}\n",
            self.done,
            self.bytes,
            last.map_or("", String::as_str)
        )
    }

    /// Picks up an earlier run of this download from its checkpoint text
    /// (`saved`) and its part file's length (`part_len`). It resumes only
    /// when the checkpoint is this format, names the same URL at the same
    /// place in this playlist, and counts no more bytes than the part file
    /// holds. Anything else starts over, because a wrong guess would join
    /// the wrong bytes into the file.
    pub fn resume(&mut self, saved: &str, part_len: u64) -> PartStart {
        let Some((done, bytes)) = self.parse_checkpoint(saved) else {
            return PartStart::Fresh;
        };
        if bytes > part_len {
            return PartStart::Fresh;
        }
        self.done = done;
        self.bytes = bytes;
        PartStart::Resume { keep: bytes }
    }

    /// `(done, bytes)` from a checkpoint of this playlist, or None.
    fn parse_checkpoint(&self, saved: &str) -> Option<(usize, u64)> {
        let mut lines = saved.lines();
        if lines.next()? != CHECKPOINT_HEADER {
            return None;
        }
        let done: usize = lines.next()?.strip_prefix("done ")?.parse().ok()?;
        let bytes: u64 = lines.next()?.strip_prefix("bytes ")?.parse().ok()?;
        let last = lines.next()?.strip_prefix("last ")?;
        let same_url = self.urls.get(done.checked_sub(1)?)? == last;
        same_url.then_some((done, bytes))
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
