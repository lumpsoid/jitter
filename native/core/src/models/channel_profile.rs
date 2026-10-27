use super::Channel;

/// What the channel page shows of a channel: the channel itself and its
/// own description. Only the channel page asks for it; favorites, streams
/// and chat carry the bare [`Channel`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelProfile {
    pub channel: Channel,
    /// The channel's "about" text; `None` when it has none.
    pub description: Option<String>,
}
