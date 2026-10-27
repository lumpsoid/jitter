//! Chat state: a bounded buffer of messages and user notices, the room's
//! modes and the emote set.
//! Heads feed it `ChatEvent`s from twitch and emotes from the providers.

use std::collections::VecDeque;

use crate::models::{
    ChatChange, ChatEntry, ChatEvent, ChatMessage, Emote, EmoteProvider, EmoteScope, EmoteSet,
    Fragment, RoomState,
};
use crate::tokenize::tokenize;

/// Oldest entries (messages and user notices) are dropped beyond this
/// many.
pub const MESSAGE_LIMIT: usize = 600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Connection {
    Disconnected,
    Connected,
}

#[derive(Debug)]
pub struct ChatController {
    entries: VecDeque<ChatEntry>,
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
            entries: VecDeque::new(),
            room: RoomState::default(),
            emotes: EmoteSet::new(),
            failed_providers: Vec::new(),
            connection: Connection::Disconnected,
            notice: None,
        }
    }

    /// Oldest first.
    pub fn entries(&self) -> impl Iterator<Item = &ChatEntry> {
        self.entries.iter()
    }

    pub fn room(&self) -> RoomState {
        self.room
    }

    pub fn connection(&self) -> Connection {
        self.connection
    }

    /// The latest NOTICE text, if any: the server's word about the room.
    /// User notices are entries instead.
    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }

    /// Providers whose emotes failed to load, so "fewer emotes" is visible.
    pub fn failed_providers(&self) -> &[EmoteProvider] {
        &self.failed_providers
    }

    /// Applies one event and returns what changed, in order; empty when
    /// nothing did (e.g. clearing a message no longer in the buffer).
    pub fn apply(&mut self, event: ChatEvent) -> Vec<ChatChange> {
        match event {
            ChatEvent::Message(message) => self.append(ChatEntry::Message(message)),
            ChatEvent::UserNotice(notice) => self.append(ChatEntry::UserNotice(notice)),
            ChatEvent::ClearMessage { id } => self.mark(|m| m.id == id),
            ChatEvent::ClearUser { user_id } => self.mark(|m| m.sender.user_id == user_id),
            ChatEvent::ClearAll => {
                self.entries.clear();
                vec![ChatChange::Cleared]
            }
            ChatEvent::RoomState(update) => {
                self.room.apply(update);
                vec![ChatChange::RoomChanged]
            }
            ChatEvent::Notice(text) => {
                self.notice = Some(text);
                vec![ChatChange::NoticeChanged]
            }
            ChatEvent::Connected => {
                self.connection = Connection::Connected;
                vec![ChatChange::ConnectionChanged]
            }
            ChatEvent::Disconnected => {
                self.connection = Connection::Disconnected;
                vec![ChatChange::ConnectionChanged]
            }
        }
    }

    /// Adds an entry at the newest end, dropping the oldest beyond
    /// [`MESSAGE_LIMIT`].
    fn append(&mut self, entry: ChatEntry) -> Vec<ChatChange> {
        self.entries.push_back(entry.clone());
        let mut changes = vec![ChatChange::Appended { entry }];
        let count = self.entries.len().saturating_sub(MESSAGE_LIMIT);
        if count > 0 {
            self.entries.drain(..count);
            changes.push(ChatChange::Evicted { count });
        }
        changes
    }

    /// Marks the matching messages not yet deleted, a user notice's own
    /// message among them; their ids, if any. A notice's system line stays.
    fn mark(&mut self, matches: impl Fn(&ChatMessage) -> bool) -> Vec<ChatChange> {
        let ids: Vec<String> = self
            .entries
            .iter_mut()
            .filter_map(ChatEntry::message_mut)
            .filter(|m| !m.deleted && matches(m))
            .map(|m| {
                m.deleted = true;
                m.id.clone()
            })
            .collect();
        if ids.is_empty() {
            Vec::new()
        } else {
            vec![ChatChange::Marked { ids }]
        }
    }

    /// Replaces one provider's emotes for a scope. May arrive at any time;
    /// a successful load clears that provider's failure mark. Messages are
    /// tokenized at read time, so every one may now render differently.
    pub fn set_emotes(
        &mut self,
        scope: EmoteScope,
        provider: EmoteProvider,
        emotes: Vec<Emote>,
    ) -> ChatChange {
        self.emotes.set(scope, provider, emotes);
        self.failed_providers.retain(|p| *p != provider);
        ChatChange::Rerender
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
