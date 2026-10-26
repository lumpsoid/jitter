//! Chat state: a bounded message buffer, the room's modes and the emote set.
//! Heads feed it `ChatEvent`s from twitch and emotes from the providers.

use std::collections::VecDeque;

use crate::models::{
    ChatEvent, ChatMessage, Emote, EmoteProvider, EmoteScope, EmoteSet, Fragment, RoomState,
};
use crate::tokenize::tokenize;

/// Oldest messages are dropped beyond this many.
pub const MESSAGE_LIMIT: usize = 600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Connection {
    Disconnected,
    Connected,
}

#[derive(Debug)]
pub struct ChatController {
    messages: VecDeque<ChatMessage>,
    room: RoomState,
    emotes: EmoteSet,
    failed_providers: Vec<EmoteProvider>,
    connection: Connection,
    notice: Option<String>,
}

impl Default for ChatController {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatController {
    pub fn new() -> Self {
        Self {
            messages: VecDeque::new(),
            room: RoomState::default(),
            emotes: EmoteSet::new(),
            failed_providers: Vec::new(),
            connection: Connection::Disconnected,
            notice: None,
        }
    }

    pub fn messages(&self) -> impl Iterator<Item = &ChatMessage> {
        self.messages.iter()
    }

    pub fn room(&self) -> RoomState {
        self.room
    }

    pub fn connection(&self) -> Connection {
        self.connection
    }

    /// The latest NOTICE text, if any.
    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }

    /// Providers whose emotes failed to load, so "fewer emotes" is visible.
    pub fn failed_providers(&self) -> &[EmoteProvider] {
        &self.failed_providers
    }

    pub fn apply(&mut self, event: ChatEvent) {
        match event {
            ChatEvent::Message(message) => {
                self.messages.push_back(message);
                while self.messages.len() > MESSAGE_LIMIT {
                    self.messages.pop_front();
                }
            }
            ChatEvent::ClearMessage { id } => {
                for message in self.messages.iter_mut().filter(|m| m.id == id) {
                    message.deleted = true;
                }
            }
            ChatEvent::ClearUser { user_id } => {
                for message in self
                    .messages
                    .iter_mut()
                    .filter(|m| m.sender.user_id == user_id)
                {
                    message.deleted = true;
                }
            }
            ChatEvent::ClearAll => self.messages.clear(),
            ChatEvent::RoomState(update) => self.room.apply(update),
            ChatEvent::Notice(text) => self.notice = Some(text),
            ChatEvent::Connected => self.connection = Connection::Connected,
            ChatEvent::Disconnected => self.connection = Connection::Disconnected,
        }
    }

    /// Replaces one provider's emotes for a scope. May arrive at any time;
    /// a successful load clears that provider's failure mark.
    pub fn set_emotes(&mut self, scope: EmoteScope, provider: EmoteProvider, emotes: Vec<Emote>) {
        self.emotes.set(scope, provider, emotes);
        self.failed_providers.retain(|p| *p != provider);
    }

    pub fn provider_failed(&mut self, provider: EmoteProvider) {
        if !self.failed_providers.contains(&provider) {
            self.failed_providers.push(provider);
        }
    }

    /// Tokenized at read time, so messages that arrived before emotes loaded
    /// render with them once they have.
    pub fn fragments(&self, message: &ChatMessage) -> Vec<Fragment> {
        tokenize(&message.text, &message.emotes, &self.emotes)
    }
}
