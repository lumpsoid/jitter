use super::ChatMessage;

/// What one chat action changed, for a head to pass on as a list diff
/// without deciding anything. The buffer is ordered oldest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatChange {
    /// A message was added at the newest end.
    Appended { message: ChatMessage },
    /// The `count` oldest messages were dropped.
    Evicted { count: usize },
    /// These messages, by `ChatMessage::id`, are now marked deleted.
    Marked { ids: Vec<String> },
    /// Every message was removed.
    Cleared,
    /// Emotes changed, so any message's fragments may now differ.
    Rerender,
    /// The room's modes changed; the controller holds them.
    RoomChanged,
    /// The connection changed; the controller holds it.
    ConnectionChanged,
    /// A NOTICE arrived; the controller holds its text.
    NoticeChanged,
}
