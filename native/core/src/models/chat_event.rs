use super::{ChannelId, ChatMessage, RoomStateUpdate, UserNotice};

/// What twitch produces from the chat connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatEvent {
    Message(ChatMessage),
    /// A sub, raid, watch streak and the like: a chat entry of its own.
    UserNotice(UserNotice),
    ClearMessage {
        id: String,
    },
    ClearUser {
        user_id: ChannelId,
    },
    ClearAll,
    RoomState(RoomStateUpdate),
    /// The server's word about the room (e.g. that it is in slow mode),
    /// not a chat entry.
    Notice(String),
    Connected,
    Disconnected,
}
