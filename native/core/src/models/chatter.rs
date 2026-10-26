use super::{Badge, ChannelId, Rgb};

/// Who sent a chat message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chatter {
    pub user_id: ChannelId,
    pub display_name: String,
    /// The colour the user chose; `None` if they never set one.
    pub color: Option<Rgb>,
    pub badges: Vec<Badge>,
}
