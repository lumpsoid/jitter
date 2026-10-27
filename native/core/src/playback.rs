//! Resolving a live channel or a VOD into a playable stream and choosing a
//! quality, then deciding what the player opens. Pure state; heads resolve
//! through twitch and hand the result in.

use crate::models::{ChannelLogin, Open, Playback, Quality, Seconds, Timestamp, VideoId};
use crate::ports::StoreError;
use crate::resume::ResumeController;
use crate::resume_policy::ResumeDecision;

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
    /// of channel B. Returns whether it applied, i.e. whether the result is
    /// the one now shown.
    pub fn apply(&mut self, target: PlaybackTarget, result: Result<Playback, String>) -> bool {
        let PlaybackState::Resolving { target: current } = &self.state else {
            return false;
        };
        if *current != target {
            return false;
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
        true
    }

    /// Selects a quality by name or by [`Playback::DEFAULT_QUALITY`].
    /// Returns whether `name` is known; an unknown name changes nothing.
    pub fn select(&mut self, name: &str) -> bool {
        let PlaybackState::Ready {
            playback, selected, ..
        } = &mut self.state
        else {
            return false;
        };
        match playback.quality(name) {
            Some(q) => {
                *selected = q.name.clone();
                true
            }
            None => false,
        }
    }

    /// What to open to start playing the selected quality: a VOD at its
    /// resume position, a live stream at the live edge. `None` until ready.
    pub fn open(&self, resume: &ResumeController) -> Result<Option<Open>, StoreError> {
        let start = match &self.state {
            PlaybackState::Ready {
                target: PlaybackTarget::Vod(id),
                ..
            } => resume.start_position(id)?,
            _ => Seconds::ZERO,
        };
        Ok(self.open_at(start))
    }

    /// Switches to quality `name` mid-play: what to open to carry on from
    /// `position`. `None`, changing nothing, when not ready or `name` is
    /// unknown.
    pub fn switch_quality(&mut self, name: &str, position: Seconds) -> Option<Open> {
        if self.select(name) {
            self.open_at(position)
        } else {
            None
        }
    }

    /// The selected quality opened at `vod_start`; live always opens at the
    /// live edge.
    fn open_at(&self, vod_start: Seconds) -> Option<Open> {
        let PlaybackState::Ready { target, .. } = &self.state else {
            return None;
        };
        let live = matches!(target, PlaybackTarget::Live(_));
        Some(Open {
            url: self.selected_quality()?.url.clone(),
            start: if live { Seconds::ZERO } else { vod_start },
            live,
        })
    }

    /// Reports that the player showing `target` is at `position` of
    /// `duration`. Recorded only while `target` is the VOD being played: a
    /// live stream has no resume point, and a report from a player left
    /// behind by a later [`Self::begin`] must not land on the new VOD. A
    /// zero `duration` is a player that has not loaded yet, whose position
    /// means nothing. `None` when nothing was recorded.
    pub fn record(
        &self,
        resume: &mut ResumeController,
        target: &PlaybackTarget,
        position: Seconds,
        duration: Seconds,
        now: Timestamp,
    ) -> Result<Option<ResumeDecision>, StoreError> {
        let PlaybackState::Ready {
            target: current @ PlaybackTarget::Vod(id),
            ..
        } = &self.state
        else {
            return Ok(None);
        };
        if current != target || duration == Seconds::ZERO {
            return Ok(None);
        }
        resume.record(id.clone(), position, duration, now).map(Some)
    }

    /// The player showing `target` was left: back to idle, unless a later
    /// [`Self::begin`] replaced `target` (its page opened before this one
    /// closed). Returns whether it left, i.e. whether the player stops.
    pub fn leave(&mut self, target: &PlaybackTarget) -> bool {
        match &self.state {
            PlaybackState::Resolving { target: current }
            | PlaybackState::Ready {
                target: current, ..
            } if current != target => false,
            _ => {
                self.state = PlaybackState::Idle;
                true
            }
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
