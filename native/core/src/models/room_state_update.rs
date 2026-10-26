use super::{RoomState, Seconds};

/// A ROOMSTATE that carries only the fields that changed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RoomStateUpdate {
    pub emote_only: Option<bool>,
    pub subs_only: Option<bool>,
    pub r9k: Option<bool>,
    pub slow: Option<Seconds>,
}

impl RoomState {
    /// Changes only the fields `update` carries.
    pub fn apply(&mut self, update: RoomStateUpdate) {
        self.emote_only = update.emote_only.unwrap_or(self.emote_only);
        self.subs_only = update.subs_only.unwrap_or(self.subs_only);
        self.r9k = update.r9k.unwrap_or(self.r9k);
        self.slow = update.slow.unwrap_or(self.slow);
    }
}
