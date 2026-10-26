use std::fmt;

const MAX_LEN: usize = 25;

/// A Twitch login: lowercase `[a-z0-9_]`, 1 to 25 characters. Invalid logins
/// never reach usher or IRC.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChannelLogin(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelLoginError {
    Empty,
    TooLong,
    InvalidChar(char),
}

impl ChannelLogin {
    pub fn parse(input: &str) -> Result<Self, ChannelLoginError> {
        if input.is_empty() {
            return Err(ChannelLoginError::Empty);
        }
        if input.chars().count() > MAX_LEN {
            return Err(ChannelLoginError::TooLong);
        }
        let lower = input.to_lowercase();
        match lower
            .chars()
            .find(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_'))
        {
            Some(c) => Err(ChannelLoginError::InvalidChar(c)),
            None => Ok(Self(lower)),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ChannelLogin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
