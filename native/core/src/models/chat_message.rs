use super::chatter::handle;
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

impl ChatMessage {
    /// The handles (see `Chatter::handle`) the text `@` mentions, each once,
    /// in the order written. A word is a mention when it starts with `@`;
    /// punctuation after the name (`@bob,`, `@bob:`) is not part of it.
    pub fn mentions(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for word in self.text.split_whitespace() {
            let Some(name) = word.strip_prefix('@') else {
                continue;
            };
            let name = name.trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_');
            let name = handle(name);
            if !name.is_empty() && !out.contains(&name) {
                out.push(name);
            }
        }
        out
    }
}
