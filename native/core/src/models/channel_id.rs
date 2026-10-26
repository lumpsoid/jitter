use std::fmt;

/// A Twitch channel (user) id: a non-empty string of ASCII digits.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChannelId(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelIdError {
    Empty,
    NotNumeric,
}

impl ChannelId {
    pub fn parse(input: &str) -> Result<Self, ChannelIdError> {
        if input.is_empty() {
            return Err(ChannelIdError::Empty);
        }
        if !input.chars().all(|c| c.is_ascii_digit()) {
            return Err(ChannelIdError::NotNumeric);
        }
        Ok(Self(input.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ChannelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
