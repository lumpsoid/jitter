use super::{ChannelId, ChannelLogin};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Channel {
    pub id: ChannelId,
    pub login: ChannelLogin,
    pub display_name: String,
    pub avatar_url: String,
}
