use super::{Channel, Timestamp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stream {
    pub id: String,
    pub channel: Channel,
    pub title: String,
    pub game_name: Option<String>,
    pub viewers: u32,
    /// When the stream went live. `None` when the source omits it (top
    /// streams carry no start time).
    pub started_at: Option<Timestamp>,
    /// Either Twitch's `{width}x{height}` template or a fixed-size URL.
    pub thumbnail_url: String,
}

impl Stream {
    /// The thumbnail URL with the `{width}x{height}` template filled in; a
    /// URL without the template is returned unchanged.
    pub fn thumbnail(&self, width: u32, height: u32) -> String {
        self.thumbnail_url
            .replace("{width}", &width.to_string())
            .replace("{height}", &height.to_string())
    }
}
