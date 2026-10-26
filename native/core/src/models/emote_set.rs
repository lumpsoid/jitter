use std::collections::HashMap;

use super::{Emote, EmoteProvider};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmoteScope {
    Global,
    Channel,
}

/// Lookup priority when two emotes share a name: channel over global, then
/// 7TV, then BTTV, then FFZ.
const PRIORITY: [(EmoteScope, EmoteProvider); 6] = [
    (EmoteScope::Channel, EmoteProvider::SevenTv),
    (EmoteScope::Channel, EmoteProvider::Bttv),
    (EmoteScope::Channel, EmoteProvider::Ffz),
    (EmoteScope::Global, EmoteProvider::SevenTv),
    (EmoteScope::Global, EmoteProvider::Bttv),
    (EmoteScope::Global, EmoteProvider::Ffz),
];

/// Third-party emotes by name. Each (scope, provider) list is replaced as a
/// whole when it loads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EmoteSet {
    lists: HashMap<(EmoteScope, EmoteProvider), HashMap<String, Emote>>,
}

impl EmoteSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, scope: EmoteScope, provider: EmoteProvider, emotes: Vec<Emote>) {
        let by_name = emotes.into_iter().map(|e| (e.name.clone(), e)).collect();
        self.lists.insert((scope, provider), by_name);
    }

    pub fn get(&self, name: &str) -> Option<&Emote> {
        PRIORITY
            .iter()
            .find_map(|key| self.lists.get(key)?.get(name))
    }
}
