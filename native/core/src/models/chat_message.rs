use super::{Chatter, EmoteSpan, Timestamp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    /// Twitch's id for the message: never empty, never changes, and what
    /// moderation and `ChatChange` refer to it by, so a GUI can key on it.
    pub id: String,
    pub sender: Chatter,
    /// Without the ACTION wrapper; `action` says it was a `/me`.
    pub text: String,
    pub action: bool,
    pub emotes: Vec<EmoteSpan>,
    pub sent_at: Timestamp,
    /// Set by the controller when a moderator removes the message.
    pub deleted: bool,
}
