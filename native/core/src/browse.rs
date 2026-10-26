//! The top-streams list: paging state, an in-flight guard and stale-result
//! protection. Pure state; heads fetch and hand results back.

use std::collections::HashSet;

use crate::models::{Cursor, Page, Stream};

/// Loading state of the page after the loaded ones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MorePages {
    Idle,
    Loading,
    Failed { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirstPage {
    Loading,
    Failed { message: String },
}

/// A paged list. Loaded items survive paging and a failed next page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListState<T> {
    Idle,
    FirstPage(FirstPage),
    Loaded {
        items: Vec<T>,
        next: Option<Cursor>,
        more: MorePages,
    },
}

/// What the head must fetch. Handed back unchanged with the result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageRequest {
    generation: u64,
    pub cursor: Option<Cursor>,
}

#[derive(Debug)]
pub struct BrowseController {
    state: ListState<Stream>,
    generation: u64,
}

impl Default for BrowseController {
    fn default() -> Self {
        Self::new()
    }
}

impl BrowseController {
    pub fn new() -> Self {
        Self {
            state: ListState::Idle,
            generation: 0,
        }
    }

    pub fn state(&self) -> &ListState<Stream> {
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
    /// the state is not waiting for it. Streams already listed are dropped:
    /// live cursors shift as viewer counts move.
    pub fn apply_page(&mut self, request: PageRequest, result: Result<Page<Stream>, String>) {
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

fn append_new(items: &mut Vec<Stream>, incoming: Vec<Stream>) {
    let mut seen: HashSet<String> = items.iter().map(|s| s.id.clone()).collect();
    items.extend(incoming.into_iter().filter(|s| seen.insert(s.id.clone())));
}
