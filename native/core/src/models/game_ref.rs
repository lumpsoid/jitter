use super::CategoryId;

/// The category a stream is in, enough to link to it: its id and its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameRef {
    pub id: CategoryId,
    pub name: String,
}
