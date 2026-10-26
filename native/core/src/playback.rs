//! Resolving a live channel or a VOD into a playable stream and choosing a
//! quality. Pure state; heads resolve through twitch and hand the result in.

use crate::models::{ChannelLogin, Playback, Quality, VideoId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackTarget {
    Live(ChannelLogin),
    Vod(VideoId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackState {
    Idle,
    Resolving {
        target: PlaybackTarget,
    },
    Ready {
        target: PlaybackTarget,
        playback: Playback,
        /// Name of the selected quality; always one of `playback.qualities`.
        selected: String,
    },
    Failed {
        message: String,
    },
}

#[derive(Debug)]
pub struct PlaybackController {
    state: PlaybackState,
}

impl Default for PlaybackController {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaybackController {
    pub fn new() -> Self {
        Self {
            state: PlaybackState::Idle,
        }
    }

    pub fn state(&self) -> &PlaybackState {
        &self.state
    }

    /// Starts resolving `target`, replacing whatever was there.
    pub fn begin(&mut self, target: PlaybackTarget) {
        self.state = PlaybackState::Resolving { target };
    }

    /// Applies a resolve result. Ignored unless `target` is the one being
    /// resolved, so a slow resolve for channel A cannot replace a later pick
    /// of channel B.
    pub fn apply(&mut self, target: PlaybackTarget, result: Result<Playback, String>) {
        let PlaybackState::Resolving { target: current } = &self.state else {
            return;
        };
        if *current != target {
            return;
        }
        self.state = match result {
            Err(message) => PlaybackState::Failed { message },
            Ok(playback) => match playback.default_quality() {
                None => PlaybackState::Failed {
                    message: "no playable qualities".to_owned(),
                },
                Some(q) => PlaybackState::Ready {
                    target,
                    selected: q.name.clone(),
                    playback,
                },
            },
        };
    }

    /// Selects a quality by name; an unknown name changes nothing.
    pub fn select(&mut self, name: &str) {
        if let PlaybackState::Ready {
            playback, selected, ..
        } = &mut self.state
            && playback.qualities.iter().any(|q| q.name == name)
        {
            *selected = name.to_owned();
        }
    }

    pub fn selected_quality(&self) -> Option<&Quality> {
        match &self.state {
            PlaybackState::Ready {
                playback, selected, ..
            } => playback.qualities.iter().find(|q| q.name == *selected),
            _ => None,
        }
    }
}
