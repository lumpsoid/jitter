use std::fmt;

/// A Twitch video id: a non-empty string of ASCII digits.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VideoId(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoIdError {
    Empty,
    NotNumeric,
}

impl VideoId {
    pub fn parse(input: &str) -> Result<Self, VideoIdError> {
        if input.is_empty() {
            return Err(VideoIdError::Empty);
        }
        if !input.chars().all(|c| c.is_ascii_digit()) {
            return Err(VideoIdError::NotNumeric);
        }
        Ok(Self(input.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for VideoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
