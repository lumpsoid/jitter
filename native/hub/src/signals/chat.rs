use rinf::{DartSignal, RustSignal, SignalPiece};
use serde::{Deserialize, Serialize};

/// The chat holder is subscribed: answer with the panel as it stands
/// (`Snapshot`, or `Off` while no live channel plays).
#[derive(Deserialize, DartSignal)]
pub struct ChatReady;

/// The GUI's brightness. Rust picks name colours readable on it and
/// resends the panel.
#[derive(Deserialize, DartSignal)]
pub struct SetTheme {
    pub dark: bool,
}

/// The user closed the chat panel: Rust leaves the chat and sends
/// `Hidden`. A hidden chat stays hidden on the next player page.
#[derive(Deserialize, DartSignal)]
pub struct HideChat;

/// The user opened the chat panel again: Rust rejoins the chat of the
/// live channel playing.
#[derive(Deserialize, DartSignal)]
pub struct ShowChat;

/// The chat panel. One type, so a snapshot and the diffs after it arrive
/// in order. Messages are oldest first.
#[derive(Serialize, RustSignal)]
pub enum ChatUpdate {
    /// No live channel plays: there is no chat to show.
    Off,
    /// A live channel plays but the user hid its chat: no panel, and an
    /// action to show it.
    Hidden,
    /// Replaces the messages and the status.
    Snapshot {
        messages: Vec<ChatLine>,
        status: ChatStatus,
    },
    /// New messages at the newest end, gathered over a short window.
    Appended { messages: Vec<ChatLine> },
    /// Drop the `count` oldest messages.
    Evicted { count: u32 },
    /// These messages were removed by a moderator; keep them, shown as
    /// deleted.
    Marked { ids: Vec<String> },
    /// Every message was removed.
    Cleared,
    /// Replaces the status; the messages stay.
    Status { status: ChatStatus },
}

/// One chat message as the panel draws it.
#[derive(Serialize, SignalPiece)]
pub struct ChatLine {
    /// Twitch's message id: unique, never changes; `Marked` refers to it.
    pub id: String,
    pub name: String,
    /// The name's colour, `0xAARRGGBB`, readable on the theme last set.
    pub color: u32,
    /// A `/me` message: the body is drawn in the name's colour.
    pub action: bool,
    pub deleted: bool,
    pub fragments: Vec<ChatFragment>,
}

/// One piece of a message body, in order.
#[derive(Serialize, SignalPiece)]
pub enum ChatFragment {
    Text {
        text: String,
    },
    Emote {
        emote: EmoteImage,
    },
    /// `overlays` are drawn over `base`, centred on it.
    Overlay {
        base: EmoteImage,
        overlays: Vec<EmoteImage>,
    },
    /// `text` as written; `url` is what opens when it is tapped.
    Link {
        text: String,
        url: String,
    },
}

#[derive(Serialize, SignalPiece)]
pub struct EmoteImage {
    pub name: String,
    pub url: String,
}

/// What the panel shows around the messages.
#[derive(Serialize, SignalPiece)]
pub struct ChatStatus {
    /// Shown below the messages while not connected; none when connected.
    pub banner: Option<String>,
    /// The room's modes in effect, e.g. `Slow mode (30s)`; empty when none.
    pub modes: Vec<String>,
    /// The latest notice from Twitch, if any.
    pub notice: Option<String>,
    /// Shown when an emote provider failed to load, so missing emotes are
    /// explained.
    pub emote_hint: Option<String>,
}
