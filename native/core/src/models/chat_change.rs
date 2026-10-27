use super::ChatEntry;

/// What one chat action changed, for a head to pass on as a list diff
/// without deciding anything. The buffer is ordered oldest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatChange {
    /// An entry was added at the newest end.
    Appended { entry: ChatEntry },
    /// The `count` oldest entries were dropped.
    Evicted { count: usize },
    /// These messages, by `ChatMessage::id`, are now marked deleted; a
    /// user notice carrying one keeps its system line.
    Marked { ids: Vec<String> },
    /// Every entry was removed.
    Cleared,
    /// Emotes changed, so any message's fragments may now differ.
    Rerender,
    /// The room's modes changed; the controller holds them.
    RoomChanged,
    /// The connection changed; the controller holds it. For a VOD's
    /// replay, whether it waits for a page or has failed changed.
    ConnectionChanged,
    /// A NOTICE arrived; the controller holds its text.
    NoticeChanged,
}
