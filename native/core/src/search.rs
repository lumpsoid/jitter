//! Search: the channels and the categories matching one query, each paged
//! on the shared [`PagedList`]. A new query starts both lists over, so a
//! page still in flight for the query before is ignored.

use crate::models::{
    Category, FoundChannel, ListState, Page, PageRequest, SearchKind, SearchQuery, SearchRequests,
};
use crate::paged_list::PagedList;

#[derive(Debug, Default)]
pub struct SearchController {
    query: Option<SearchQuery>,
    channels: PagedList<FoundChannel>,
    categories: PagedList<Category>,
}

impl SearchController {
    /// No query and nothing loaded until the first `search`.
    pub fn new() -> Self {
        Self {
            query: None,
            channels: PagedList::new(),
            categories: PagedList::new(),
        }
    }

    /// The query both lists are for; `None` before a search and after
    /// `clear`.
    pub fn query(&self) -> Option<&SearchQuery> {
        self.query.as_ref()
    }

    pub fn channels(&self) -> &ListState<FoundChannel> {
        self.channels.state()
    }

    pub fn categories(&self) -> &ListState<Category> {
        self.categories.state()
    }

    /// Switches to `query` and starts both lists over from their first
    /// page. `None` when `query` is the one already shown: its lists stay.
    pub fn search(&mut self, query: SearchQuery) -> Option<SearchRequests> {
        if self.query.as_ref() == Some(&query) {
            return None;
        }
        self.query = Some(query);
        Some(SearchRequests {
            channels: self.channels.begin_refresh(),
            categories: self.categories.begin_refresh(),
        })
    }

    /// Forgets the query: both lists go back to idle and any page in
    /// flight is ignored.
    pub fn clear(&mut self) {
        self.query = None;
        self.channels.clear();
        self.categories.clear();
    }

    /// Starts list `kind` over from its first page; `None` without a query.
    pub fn begin_refresh(&mut self, kind: SearchKind) -> Option<PageRequest> {
        self.query.as_ref()?;
        Some(match kind {
            SearchKind::Channels => self.channels.begin_refresh(),
            SearchKind::Categories => self.categories.begin_refresh(),
        })
    }

    /// The next page of list `kind`, as [`PagedList::begin_next`] decides.
    pub fn begin_next(&mut self, kind: SearchKind) -> Option<PageRequest> {
        match kind {
            SearchKind::Channels => self.channels.begin_next(),
            SearchKind::Categories => self.categories.begin_next(),
        }
    }

    /// A fetched page of channels; one for an earlier query is ignored.
    pub fn apply_channels(
        &mut self,
        request: PageRequest,
        result: Result<Page<FoundChannel>, String>,
    ) {
        self.channels.apply_page(request, result);
    }

    /// A fetched page of categories; one for an earlier query is ignored.
    pub fn apply_categories(
        &mut self,
        request: PageRequest,
        result: Result<Page<Category>, String>,
    ) {
        self.categories.apply_page(request, result);
    }
}
