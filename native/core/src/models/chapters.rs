use super::{Chapter, GameRef, Seconds};

/// A VOD's chapters, in the order they start. Empty for a VOD that stayed
/// in one game, a live stream, or when Twitch could not tell.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Chapters(Vec<Chapter>);

impl Chapters {
    /// `chapters` in the order they start, whatever order they came in.
    pub fn new(mut chapters: Vec<Chapter>) -> Self {
        chapters.sort_by_key(|chapter| chapter.start);
        Self(chapters)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The chapters to go to: none for a VOD in one chapter, which has
    /// nowhere else to go.
    pub fn to_go_to(&self) -> &[Chapter] {
        match self.0.len() {
            0 | 1 => &[],
            _ => &self.0,
        }
    }

    /// Where chapter `index` starts; `None` for one there is not.
    pub fn start(&self, index: usize) -> Option<Seconds> {
        self.0.get(index).map(|chapter| chapter.start)
    }

    /// The index of the chapter playing at `position`: the last one
    /// started by then; the first one before it starts.
    pub fn index_at(&self, position: Seconds) -> Option<usize> {
        if self.0.is_empty() {
            return None;
        }
        let started = self.0.partition_point(|chapter| chapter.start <= position);
        Some(started.saturating_sub(1))
    }

    /// The game being played at `position` ([`Self::index_at`]).
    pub fn game_at(&self, position: Seconds) -> Option<&GameRef> {
        let index = self.index_at(position)?;
        self.0.get(index).map(|chapter| &chapter.game)
    }
}
