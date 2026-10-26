/// A third-party emote provider. Twitch's own emotes arrive with the message
/// and are not part of an `EmoteSet`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmoteProvider {
    SevenTv,
    Bttv,
    Ffz,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Emote {
    pub id: String,
    /// The text that triggers it in a message.
    pub name: String,
    pub url: String,
    /// Drawn over the previous emote instead of beside it.
    pub zero_width: bool,
}
