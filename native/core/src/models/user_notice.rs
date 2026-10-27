use super::ChatMessage;

/// Twitch's line about something a user did in the room (a sub, a resub,
/// a gifted sub, a raid, a watch streak), shown in the chat where it
/// arrived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserNotice {
    /// Twitch's own wording, e.g. `x subscribed at Tier 1.`; never empty.
    pub system: String,
    /// What the user wrote along with it, if anything.
    pub message: Option<ChatMessage>,
}
