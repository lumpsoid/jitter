/// A third-party emote provider. Twitch's own emotes arrive with the message
/// and are not part of an `EmoteSet`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmoteProvider {
    SevenTv,
    Bttv,
    Ffz,
}

impl EmoteProvider {
    pub const ALL: [Self; 3] = [Self::SevenTv, Self::Bttv, Self::Ffz];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Emote {
    pub id: String,
    /// The text that triggers it in a message.
    pub name: String,
    /// The image, animated if the emote is.
    pub url: String,
    /// A still image of an animated emote; `None` when `url` never moves
    /// or its provider offers no still.
    pub still_url: Option<String>,
    /// Drawn over the previous emote instead of beside it.
    pub zero_width: bool,
}

impl Emote {
    /// The image to show: `url`, or, with `animated` off, the still where
    /// there is one. An emote without a still shows `url` either way.
    pub fn image_url(&self, animated: bool) -> &str {
        match (&self.still_url, animated) {
            (Some(still), false) => still,
            _ => &self.url,
        }
    }
}
