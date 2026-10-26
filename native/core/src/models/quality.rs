#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityKind {
    Source,
    Transcode,
    AudioOnly,
}

/// A selectable stream quality. `frame_rate` is whole frames per second.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quality {
    pub name: String,
    pub url: String,
    pub resolution: Option<Resolution>,
    pub frame_rate: Option<u32>,
    pub kind: QualityKind,
}
