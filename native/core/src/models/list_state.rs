use super::Cursor;

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
