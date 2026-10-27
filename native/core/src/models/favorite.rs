use super::{ChannelId, ChannelLogin, Timestamp};

/// A locally kept channel. The key is `channel_id`; `login`,
/// `display_name` and `avatar_url` are display caches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Favorite {
    pub channel_id: ChannelId,
    pub login: ChannelLogin,
    pub display_name: String,
    /// `None` until one is known: the channel was stored before avatars were
    /// kept and has not been seen live since, or Twitch sent none.
    pub avatar_url: Option<String>,
    pub added_at: Timestamp,
}
