use super::{GameRef, Stream};

/// A category some live favorites are streaming right now, with their
/// streams. Borrowed from the favorites list it was grouped from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FavoritesCategory<'a> {
    pub game: &'a GameRef,
    /// Most-watched first.
    pub streams: Vec<&'a Stream>,
}

impl FavoritesCategory<'_> {
    /// The favorites' viewers in this category, summed.
    pub fn viewers(&self) -> u64 {
        self.streams.iter().map(|s| u64::from(s.viewers)).sum()
    }
}
