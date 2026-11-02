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

impl Chatter {
    /// The name as an `@` mention of this chatter matches it: the display
    /// name, case folded, as Twitch names are matched regardless of case.
    pub fn handle(&self) -> String {
        handle(&self.display_name)
    }
}

pub(super) fn handle(name: &str) -> String {
    name.to_lowercase()
}
