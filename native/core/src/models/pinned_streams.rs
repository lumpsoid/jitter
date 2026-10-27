use super::Stream;

/// A category's streams with the favorites playing it pinned first.
/// Borrowed from the lists they were drawn from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedStreams<'a> {
    /// The live favorites in the category, most-watched first, whether or
    /// not the loaded pages have reached them.
    pub favorites: Vec<&'a Stream>,
    /// The loaded streams in page order, without the pinned channels.
    pub others: Vec<&'a Stream>,
}
