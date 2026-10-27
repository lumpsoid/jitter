use super::{ChannelId, ChannelLogin};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Channel {
    pub id: ChannelId,
    pub login: ChannelLogin,
    pub display_name: String,
    pub avatar_url: String,
}

impl Channel {
    /// The avatar URL, or `None` when it is empty (no avatar known).
    pub fn avatar(&self) -> Option<&str> {
        Some(self.avatar_url.as_str()).filter(|url| !url.is_empty())
    }
}
