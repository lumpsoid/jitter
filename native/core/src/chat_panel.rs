//! Whether the player page's chat runs. A chat session runs while a live
//! channel or a VOD plays and the user has not hidden the panel: a live
//! channel's chat, or a VOD's chat replay. Hiding it leaves the chat,
//! showing it again starts a fresh session. The choice outlives one player
//! page: a chat hidden on one channel stays hidden on the next. With the
//! auto-load setting off, every player page starts with the chat hidden.
//! Pure state; the head starts and ends sessions as told.

use crate::models::{ChannelLogin, VideoId};
use crate::playback::PlaybackTarget;

/// What the panel shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatPanelState {
    /// Nothing plays: there is no chat to show or hide.
    Off,
    /// Something plays and the user hid its chat.
    Hidden,
    /// The chat of what plays is shown.
    Shown,
}

/// What the head does with the chat session after an action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionChange {
    /// Start a new session on this channel's live chat, replacing any
    /// running one.
    Join(ChannelLogin),
    /// Start a new session replaying this VOD's chat, replacing any running
    /// one.
    Replay(VideoId),
    /// End the running session, if any.
    Leave,
    /// Leave the session as it is.
    Keep,
}

#[derive(Debug)]
pub struct ChatPanelController {
    /// What plays, if anything.
    playing: Option<PlaybackTarget>,
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
            playing: None,
            hidden: false,
        }
    }

    pub fn state(&self) -> ChatPanelState {
        match (&self.playing, self.hidden) {
            (None, _) => ChatPanelState::Off,
            (Some(_), true) => ChatPanelState::Hidden,
            (Some(_), false) => ChatPanelState::Shown,
        }
    }

    /// A player page opened on `target` (`None`: playback rejected it). A
    /// live channel joins its chat and a VOD replays its own, unless the
    /// panel is hidden; a rejected target leaves. Without `auto_load` (the
    /// user's setting) the panel starts hidden, so nothing is joined until
    /// the user shows it. The target that already plays (the page's Retry,
    /// say after the channel went offline) keeps the chat as it is: only
    /// the player reconnects.
    pub fn play(&mut self, target: Option<&PlaybackTarget>, auto_load: bool) -> SessionChange {
        if target.is_some() && self.playing.as_ref() == target {
            return SessionChange::Keep;
        }
        self.playing = target.cloned();
        if !auto_load {
            self.hidden = true;
        }
        self.session()
    }

    /// The page swapped the stream it plays for `stream`, as a live page
    /// rewinding into its recording or going back: the chat follows, a
    /// recording's replay or the channel's live chat. Unlike [`Self::play`]
    /// it keeps the panel as the user left it, whatever the auto-load
    /// setting.
    pub fn switch(&mut self, stream: &PlaybackTarget) -> SessionChange {
        self.playing = Some(stream.clone());
        self.session()
    }

    /// The player stopped: no chat to show.
    pub fn stop(&mut self) -> SessionChange {
        self.playing = None;
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

    /// The user opened the panel again: rejoins the chat of what plays, if
    /// anything.
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
        match (&self.playing, self.hidden) {
            (Some(PlaybackTarget::Live(login)), false) => SessionChange::Join(login.clone()),
            (Some(PlaybackTarget::Vod(video)), false) => SessionChange::Replay(video.clone()),
            _ => SessionChange::Leave,
        }
    }
}
