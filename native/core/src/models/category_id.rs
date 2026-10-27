use std::fmt;

/// A Twitch category (game) id: a non-empty string of ASCII digits.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CategoryId(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoryIdError {
    Empty,
    NotNumeric,
}

impl CategoryId {
    pub fn parse(input: &str) -> Result<Self, CategoryIdError> {
        if input.is_empty() {
            return Err(CategoryIdError::Empty);
        }
        if !input.chars().all(|c| c.is_ascii_digit()) {
            return Err(CategoryIdError::NotNumeric);
        }
        Ok(Self(input.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CategoryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
