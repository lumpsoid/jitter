use super::{Cursor, Seconds};

/// Where a page of a VOD's chat starts: at a position in the video, or
/// right after an earlier page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayFrom {
    Offset(Seconds),
    After(Cursor),
}
