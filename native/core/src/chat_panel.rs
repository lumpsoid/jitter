//! Whether the player page's chat runs. A chat session runs while a live
//! channel plays and the user has not hidden the panel; hiding it leaves
//! the chat, showing it again joins a fresh session. The choice outlives
//! one player page: a chat hidden on one channel stays hidden on the next.
//! Pure state; the head starts and ends sessions as told.

use crate::models::ChannelLogin;
use crate::playback::PlaybackTarget;

/// What the panel shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatPanelState {
    /// No live channel plays: there is no chat to show or hide.
    Off,
    /// A live channel plays and the user hid its chat.
    Hidden,
    /// The live channel's chat is shown.
    Shown,
}

/// What the head does with the chat session after an action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionChange {
    /// Start a new session on this channel, replacing any running one.
    Join(ChannelLogin),
    /// End the running session, if any.
    Leave,
    /// Leave the session as it is.
    Keep,
}

#[derive(Debug)]
pub struct ChatPanelController {
    /// The live channel playing, if any.
    channel: Option<ChannelLogin>,
    hidden: bool,
}

impl Default for ChatPanelController {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatPanelController {
    /// Shown by default.
    pub fn new() -> Self {
        Self {
            channel: None,
            hidden: false,
        }
    }

    pub fn state(&self) -> ChatPanelState {
        match (&self.channel, self.hidden) {
            (None, _) => ChatPanelState::Off,
            (Some(_), true) => ChatPanelState::Hidden,
            (Some(_), false) => ChatPanelState::Shown,
        }
    }

    /// A player page opened on `target` (`None`: playback rejected it). A
    /// live channel joins its chat unless the panel is hidden; anything
    /// else leaves.
    pub fn play(&mut self, target: Option<&PlaybackTarget>) -> SessionChange {
        self.channel = match target {
            Some(PlaybackTarget::Live(login)) => Some(login.clone()),
            Some(PlaybackTarget::Vod(_)) | None => None,
        };
        self.session()
    }

    /// The player stopped: no chat to show.
    pub fn stop(&mut self) -> SessionChange {
        self.channel = None;
        SessionChange::Leave
    }

    /// The user closed the panel: leaves the chat.
    pub fn hide(&mut self) -> SessionChange {
        match self.hidden {
            true => SessionChange::Keep,
            false => {
                self.hidden = true;
                SessionChange::Leave
            }
        }
    }

    /// The user opened the panel again: rejoins the chat of the channel
    /// playing, if any.
    pub fn show(&mut self) -> SessionChange {
        match self.hidden {
            false => SessionChange::Keep,
            true => {
                self.hidden = false;
                self.session()
            }
        }
    }

    /// The session the current state calls for.
    fn session(&self) -> SessionChange {
        match (&self.channel, self.hidden) {
            (Some(login), false) => SessionChange::Join(login.clone()),
            _ => SessionChange::Leave,
        }
    }
}
