use super::Cursor;

/// What the head must fetch. Handed back unchanged with the result; the
/// generation tells a page of the current list from a stale one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageRequest {
    pub(crate) generation: u64,
    pub cursor: Option<Cursor>,
}
