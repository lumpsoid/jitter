/// The two lists a search fills: channels and categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchKind {
    Channels,
    Categories,
}
