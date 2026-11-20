/// A VOD quality's media segments in play order, as absolute URLs.
/// `init` is the fMP4 initialization segment, written before the first segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentList {
    pub init: Option<String>,
    pub segments: Vec<String>,
}

impl SegmentList {
    /// The extension of the file the segments join into: `mp4` for fMP4 (an
    /// init segment is present), otherwise `ts`.
    pub fn extension(&self) -> &'static str {
        if self.init.is_some() { "mp4" } else { "ts" }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(init: Option<&str>) -> SegmentList {
        SegmentList {
            init: init.map(str::to_owned),
            segments: vec!["https://example.test/0.ts".to_owned()],
        }
    }

    #[test]
    fn an_init_segment_means_mp4() {
        assert_eq!(
            list(Some("https://example.test/init-0.mp4")).extension(),
            "mp4"
        );
    }

    #[test]
    fn without_an_init_segment_it_is_ts() {
        assert_eq!(list(None).extension(), "ts");
    }
}
