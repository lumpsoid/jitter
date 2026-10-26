use super::{ChannelId, ChannelLogin, Timestamp};

/// A locally kept channel. The key is `channel_id`; `login` and
/// `display_name` are display caches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Favorite {
    pub channel_id: ChannelId,
    pub login: ChannelLogin,
    pub display_name: String,
    pub added_at: Timestamp,
}
