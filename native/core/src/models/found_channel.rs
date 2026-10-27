use super::Channel;

/// A channel a search found, with what the results show beside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundChannel {
    pub channel: Channel,
    /// Its follower count; `None` when Twitch sent none.
    pub followers: Option<u32>,
    /// Its live viewers; `None` while it is offline.
    pub viewers: Option<u32>,
}
