use std::fmt;

/// What the user searches for: the typed text without surrounding
/// whitespace, never empty. Text with nothing in it is not searchable, so
/// it cannot be a query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchQuery(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchQueryError {
    Empty,
}

impl SearchQuery {
    pub fn parse(input: &str) -> Result<Self, SearchQueryError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(SearchQueryError::Empty);
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SearchQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
