use std::fmt;

/// A store could not read or write. Carries the backend's reason as text:
/// core never branches on why storage failed, only that it did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreError(String);

impl StoreError {
    pub fn new(reason: impl Into<String>) -> Self {
        Self(reason.into())
    }

    pub fn reason(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Storage failed: {}", self.0)
    }
}

impl std::error::Error for StoreError {}
