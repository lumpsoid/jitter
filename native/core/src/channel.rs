//! The channel page's header and About: the channel's profile looked up by
//! login, then whether it is live. Pure state; heads look the channel up
//! and check it through twitch and hand the results in. Opening another
//! channel makes results for the one before it stale.

use crate::models::{ChannelId, ChannelLogin, ChannelProfile, Stream};

/// Why a channel lookup gave no channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelLookupError {
    /// No channel has this login.
    NotFound,
    /// The lookup itself failed.
    Failed(String),
}

/// Whether the open channel is live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveStatus {
    Checking,
    Offline,
    Live(Box<Stream>),
    Failed { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelState {
    /// No channel opened yet.
    Idle,
    Loading {
        login: ChannelLogin,
    },
    NotFound {
        login: ChannelLogin,
    },
    Failed {
        login: ChannelLogin,
        message: String,
    },
    Found {
        profile: ChannelProfile,
        live: LiveStatus,
    },
}

#[derive(Debug)]
pub struct ChannelController {
    state: ChannelState,
}

impl Default for ChannelController {
    fn default() -> Self {
        Self::new()
    }
}

impl ChannelController {
    pub fn new() -> Self {
        Self {
            state: ChannelState::Idle,
        }
    }

    pub fn state(&self) -> &ChannelState {
        &self.state
    }

    /// Starts looking `login` up, replacing whatever was open.
    pub fn open(&mut self, login: ChannelLogin) {
        self.state = ChannelState::Loading { login };
    }

    /// Looks the open channel up again: the login to look up, or `None`
    /// when no channel was opened.
    pub fn refresh(&mut self) -> Option<ChannelLogin> {
        let login = match &self.state {
            ChannelState::Idle => return None,
            ChannelState::Loading { login }
            | ChannelState::NotFound { login }
            | ChannelState::Failed { login, .. } => login.clone(),
            ChannelState::Found { profile, .. } => profile.channel.login.clone(),
        };
        self.open(login.clone());
        Some(login)
    }

    /// Applies a lookup of `login`. Ignored unless `login` is being looked
    /// up. Returns the channel whose live status to check next.
    pub fn apply_channel(
        &mut self,
        login: &ChannelLogin,
        result: Result<ChannelProfile, ChannelLookupError>,
    ) -> Option<ChannelId> {
        let ChannelState::Loading { login: current } = &self.state else {
            return None;
        };
        if current != login {
            return None;
        }
        let (state, check) = match result {
            Ok(profile) => {
                let id = profile.channel.id.clone();
                let found = ChannelState::Found {
                    profile,
                    live: LiveStatus::Checking,
                };
                (found, Some(id))
            }
            Err(ChannelLookupError::NotFound) => (
                ChannelState::NotFound {
                    login: login.clone(),
                },
                None,
            ),
            Err(ChannelLookupError::Failed(message)) => (
                ChannelState::Failed {
                    login: login.clone(),
                    message,
                },
                None,
            ),
        };
        self.state = state;
        check
    }

    /// Applies a live check of channel `id`: its stream among `streams`, or
    /// offline when there is none. Ignored unless that channel is open and
    /// being checked.
    pub fn apply_live(&mut self, id: &ChannelId, result: Result<Vec<Stream>, String>) {
        let ChannelState::Found { profile, live } = &mut self.state else {
            return;
        };
        if profile.channel.id != *id || *live != LiveStatus::Checking {
            return;
        }
        *live = match result {
            Ok(streams) => streams
                .into_iter()
                .find(|s| s.channel.id == *id)
                .map_or(LiveStatus::Offline, |s| LiveStatus::Live(Box::new(s))),
            Err(message) => LiveStatus::Failed { message },
        };
    }
}
