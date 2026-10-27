//! One category's live streams, paged, with the favorites playing it
//! pinned first. Opening another category starts a new list: a page still
//! in flight for the old one is ignored.

use std::collections::HashSet;

use crate::categories::{live_stream, sort_by_viewers};
use crate::favorites::FavoriteEntry;
use crate::models::{CategoryId, ListState, Page, PageRequest, PinnedStreams, Stream};
use crate::paged_list::PagedList;

#[derive(Debug)]
pub struct CategoryController {
    id: CategoryId,
    list: PagedList<Stream>,
}

impl CategoryController {
    /// Nothing is loaded until the first `begin_refresh`.
    pub fn new(id: CategoryId) -> Self {
        Self {
            id,
            list: PagedList::new(),
        }
    }

    /// The category a request from this controller is for.
    pub fn id(&self) -> &CategoryId {
        &self.id
    }

    pub fn state(&self) -> &ListState<Stream> {
        self.list.state()
    }

    /// Switches to category `id` and starts over from its first page.
    pub fn show(&mut self, id: CategoryId) -> PageRequest {
        self.id = id;
        self.list.begin_refresh()
    }

    pub fn begin_refresh(&mut self) -> PageRequest {
        self.list.begin_refresh()
    }

    pub fn begin_next(&mut self) -> Option<PageRequest> {
        self.list.begin_next()
    }

    pub fn apply_page(&mut self, request: PageRequest, result: Result<Page<Stream>, String>) {
        self.list.apply_page(request, result);
    }
}

/// The streams of category `category` with its live favorites first.
///
/// The pinned streams come from the favorites' own live check, so a
/// favorite deep in the directory is pinned before its page loads; most
/// viewers first, ties by login. `streams` (the loaded pages) follow in
/// their order, without the pinned channels, so no channel shows twice.
pub fn pin_favorites<'a>(
    category: &CategoryId,
    favorites: &'a [FavoriteEntry],
    streams: &'a [Stream],
) -> PinnedStreams<'a> {
    let mut pinned: Vec<&Stream> = favorites
        .iter()
        .filter_map(live_stream)
        .filter(|s| s.game.as_ref().is_some_and(|g| g.id == *category))
        .collect();
    sort_by_viewers(&mut pinned);
    let channels: HashSet<_> = pinned.iter().map(|s| &s.channel.id).collect();
    let others = streams
        .iter()
        .filter(|s| !channels.contains(&s.channel.id))
        .collect();
    PinnedStreams {
        favorites: pinned,
        others,
    }
}
