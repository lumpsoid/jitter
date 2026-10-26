use super::Emote;

/// One renderable piece of a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fragment {
    Text(String),
    Emote(Emote),
    /// `overlays` are drawn over `base`.
    Overlay {
        base: Emote,
        overlays: Vec<Emote>,
    },
    Link(String),
}
