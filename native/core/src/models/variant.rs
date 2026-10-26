use super::{QualityKind, Resolution};

/// A raw master-playlist entry as twitch's `hls.rs` produces it. It carries
/// the bandwidth that the ordering rule needs and `Quality` does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub name: String,
    pub url: String,
    pub resolution: Option<Resolution>,
    pub frame_rate: Option<u32>,
    pub bandwidth: u64,
    pub kind: QualityKind,
}
