//! One VOD download: the file it writes and how far it got. The head fetches each URL in order and appends its bytes to the part file.

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

    /// Every URL to fetch, in write order: the init segment first, if any.
    pub fn urls(&self) -> &[String] {
        &self.urls
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
