use super::{Channel, Timestamp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stream {
    pub id: String,
    pub channel: Channel,
    pub title: String,
    pub game_name: Option<String>,
    pub viewers: u32,
    pub started_at: Timestamp,
    /// Twitch's template, containing `{width}x{height}`.
    pub thumbnail_url: String,
}

impl Stream {
    /// The thumbnail URL with the `{width}x{height}` template filled in.
    pub fn thumbnail(&self, width: u32, height: u32) -> String {
        self.thumbnail_url
            .replace("{width}", &width.to_string())
            .replace("{height}", &height.to_string())
    }
}
