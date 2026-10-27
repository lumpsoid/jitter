use super::PageRequest;

/// The first page of each list a new search needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchRequests {
    pub channels: PageRequest,
    pub categories: PageRequest,
}
