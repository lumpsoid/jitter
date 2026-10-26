use super::{ChannelId, ChatMessage, RoomStateUpdate};

/// What twitch produces from the chat connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatEvent {
    Message(ChatMessage),
    ClearMessage { id: String },
    ClearUser { user_id: ChannelId },
    ClearAll,
    RoomState(RoomStateUpdate),
    Notice(String),
    Connected,
    Disconnected,
}
