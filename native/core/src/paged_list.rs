//! The paging state machine every list shares: a generation per refresh, an
//! in-flight guard and dedupe by id. Pure state; heads fetch and hand
//! results back.

use std::collections::HashSet;
use std::hash::Hash;

use crate::models::{
    Category, CategoryId, ChannelId, FirstPage, FoundChannel, ListState, MorePages, Page,
    PageRequest, Stream, Video, VideoId,
};

/// An item with an id that stays the same across pages.
pub trait Keyed {
    type Key: Eq + Hash + Clone;
    fn key(&self) -> &Self::Key;
}

impl Keyed for Stream {
    type Key = String;
    fn key(&self) -> &String {
        &self.id
    }
}

impl Keyed for Category {
    type Key = CategoryId;
    fn key(&self) -> &CategoryId {
        &self.id
    }
}

impl Keyed for FoundChannel {
    type Key = ChannelId;
    fn key(&self) -> &ChannelId {
        &self.channel.id
    }
}

impl Keyed for Video {
    type Key = VideoId;
    fn key(&self) -> &VideoId {
        &self.id
    }
}

#[derive(Debug)]
pub struct PagedList<T> {
    state: ListState<T>,
    generation: u64,
}

impl<T: Keyed> Default for PagedList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Keyed> PagedList<T> {
    pub fn new() -> Self {
        Self {
            state: ListState::Idle,
            generation: 0,
        }
    }

    pub fn state(&self) -> &ListState<T> {
        &self.state
    }

    /// Starts over from the first page; results of earlier requests are
    /// ignored from now on.
    pub fn begin_refresh(&mut self) -> PageRequest {
        self.generation += 1;
        self.state = ListState::FirstPage(FirstPage::Loading);
        PageRequest {
            generation: self.generation,
            cursor: None,
        }
    }

    /// Empties the list back to `Idle`; results of earlier requests are
    /// ignored from now on.
    pub fn clear(&mut self) {
        self.generation += 1;
        self.state = ListState::Idle;
    }

    /// Checks and transitions in one call: `None` while a next page is
    /// loading, when there is none, or before the first page has loaded. A
    /// failed next page can be retried.
    pub fn begin_next(&mut self) -> Option<PageRequest> {
        let ListState::Loaded { next, more, .. } = &mut self.state else {
            return None;
        };
        if *more == MorePages::Loading {
            return None;
        }
        let cursor = next.clone()?;
        *more = MorePages::Loading;
        Some(PageRequest {
            generation: self.generation,
            cursor: Some(cursor),
        })
    }

    /// Applies a fetched page. Ignored when it belongs to an older refresh or
    /// the state is not waiting for it. Items already listed are dropped:
    /// cursors can shift while paging (live viewer counts move).
    pub fn apply_page(&mut self, request: PageRequest, result: Result<Page<T>, String>) {
        if request.generation != self.generation {
            return;
        }
        match (&mut self.state, request.cursor) {
            (ListState::FirstPage(FirstPage::Loading), None) => {
                self.state = match result {
                    Ok(page) => {
                        let mut items = Vec::new();
                        append_new(&mut items, page.items);
                        ListState::Loaded {
                            items,
                            next: page.next,
                            more: MorePages::Idle,
                        }
                    }
                    Err(message) => ListState::FirstPage(FirstPage::Failed { message }),
                };
            }
            (
                ListState::Loaded {
                    items, next, more, ..
                },
                Some(_),
            ) if *more == MorePages::Loading => match result {
                Ok(page) => {
                    append_new(items, page.items);
                    *next = page.next;
                    *more = MorePages::Idle;
                }
                Err(message) => *more = MorePages::Failed { message },
            },
            _ => {}
        }
    }
}

fn append_new<T: Keyed>(items: &mut Vec<T>, incoming: Vec<T>) {
    let mut seen: HashSet<T::Key> = items.iter().map(|i| i.key().clone()).collect();
    items.extend(
        incoming
            .into_iter()
            .filter(|i| seen.insert(i.key().clone())),
    );
}
