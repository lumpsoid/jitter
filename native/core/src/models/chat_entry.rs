use super::{ChatMessage, UserNotice};

/// One entry of the chat, in arrival order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatEntry {
    Message(ChatMessage),
    UserNotice(UserNotice),
}

impl ChatEntry {
    /// What a user wrote in this entry; none for a notice without text.
    pub fn message(&self) -> Option<&ChatMessage> {
        match self {
            Self::Message(message) => Some(message),
            Self::UserNotice(notice) => notice.message.as_ref(),
        }
    }

    pub(crate) fn message_mut(&mut self) -> Option<&mut ChatMessage> {
        match self {
            Self::Message(message) => Some(message),
            Self::UserNotice(notice) => notice.message.as_mut(),
        }
    }
}
