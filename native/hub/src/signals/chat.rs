use rinf::{DartSignal, RustSignal, SignalPiece};
use serde::{Deserialize, Serialize};

/// The chat holder is subscribed: answer with the panel as it stands
/// (`Snapshot`, or `Off` while nothing plays).
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
/// live channel playing, or replays the VOD's from where the player is.
#[derive(Deserialize, DartSignal)]
pub struct ShowChat;

/// The chat panel: a live channel's chat, or a VOD's replayed in step
/// with the player. One type, so a snapshot and the diffs after it arrive
/// in order. Messages (user notices among them) are oldest first.
#[derive(Serialize, RustSignal)]
pub enum ChatUpdate {
    /// Nothing plays: there is no chat to show.
    Off,
    /// A live channel or a VOD plays but the user hid its chat: no panel,
    /// and an action to show it.
    Hidden,
    /// Replaces the messages and the status.
    Snapshot {
        messages: Vec<ChatItem>,
        status: ChatStatus,
    },
    /// New messages at the newest end, gathered over a short window.
    Appended { messages: Vec<ChatItem> },
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

/// One entry of the chat, as the panel draws it.
#[derive(Serialize, SignalPiece)]
pub enum ChatItem {
    /// What a user wrote.
    Message { line: ChatLine },
    /// Twitch's own line about a sub, a raid, a watch streak and the like,
    /// drawn as such, then what the user wrote with it, if anything.
    Notice {
        system: String,
        line: Option<ChatLine>,
    },
}

/// One chat message as the panel draws it.
#[derive(Serialize, SignalPiece)]
pub struct ChatLine {
    /// Twitch's message id: unique, never changes; `Marked` refers to it.
    pub id: String,
    /// Who wrote it: Twitch's user id, the same on each of their lines
    /// (a display name can change mid-session), so the GUI can gather a
    /// user's lines by it.
    pub sender: String,
    pub name: String,
    /// The sender's name as `mentions` holds it, so the GUI can gather the
    /// lines that mention a user by it.
    pub handle: String,
    /// Whom the line `@` mentions, as their `handle`s.
    pub mentions: Vec<String>,
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
    /// Shown below the messages while live chat is not connected, or while
    /// a replay loads the chat for where the player is; none otherwise.
    pub banner: Option<String>,
    /// The room's modes in effect, e.g. `Slow mode (30s)`; empty when none.
    pub modes: Vec<String>,
    /// The latest notice from Twitch, or why a replay stopped, if any.
    pub notice: Option<String>,
    /// Shown when an emote provider failed to load, so missing emotes are
    /// explained.
    pub emote_hint: Option<String>,
}
