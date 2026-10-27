//! Resolving a live channel or a VOD into a playable stream and choosing a
//! quality, then deciding what the player opens. Pure state; heads resolve
//! through twitch and hand the result in.

use std::time::{Duration, Instant};

use crate::models::{
    ChannelLogin, Open, Playback, PlaybackDetails, Quality, Seconds, SleepStep, SleepTimer,
    Timestamp, VideoId,
};
use crate::ports::StoreError;
use crate::resume::ResumeController;
use crate::resume_policy::ResumeDecision;

/// A resolve still unanswered after this long has failed: heads stop
/// waiting and call [`PlaybackController::time_out`], so the page can retry.
pub const RESOLVE_TIMEOUT: Seconds = Seconds::new(20);

/// What a resolve that ran out of [`RESOLVE_TIMEOUT`] fails with.
pub const TIMED_OUT: &str = "Twitch took too long to answer";

/// The sleep timers the player page offers, in minutes.
pub const SLEEP_PRESETS: [u32; 5] = [15, 30, 45, 60, 90];

/// One [`PlaybackController::begin`]: tells it from a later begin of the
/// same target, so an old attempt's timeout cannot fail a fresh one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attempt(u64);

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
        /// What is playing, for the page to show (boxed: it outweighs the
        /// other states).
        details: Box<PlaybackDetails>,
        /// The page's sleep timer, while armed.
        sleep: Option<SleepTimer>,
    },
    /// The sleep timer stopped `target`; the page stays, offering to
    /// play it again.
    Slept {
        target: PlaybackTarget,
    },
    Failed {
        message: String,
    },
}

#[derive(Debug)]
pub struct PlaybackController {
    state: PlaybackState,
    /// The latest [`Attempt`] begun.
    attempt: Attempt,
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
            attempt: Attempt(0),
        }
    }

    pub fn state(&self) -> &PlaybackState {
        &self.state
    }

    /// Starts resolving `target`, replacing whatever was there. Returns the
    /// attempt, to hand to [`Self::time_out`] if it runs out of time.
    pub fn begin(&mut self, target: PlaybackTarget) -> Attempt {
        self.state = PlaybackState::Resolving { target };
        self.attempt = Attempt(self.attempt.0.wrapping_add(1));
        self.attempt
    }

    /// `attempt` ran out of [`RESOLVE_TIMEOUT`]: fails with [`TIMED_OUT`],
    /// unless it was answered, left, or replaced by a later [`Self::begin`]
    /// since; a result arriving after this is then ignored by
    /// [`Self::apply`]. Returns whether it failed, i.e. whether the page
    /// changed.
    pub fn time_out(&mut self, attempt: Attempt) -> bool {
        if attempt != self.attempt || !matches!(self.state, PlaybackState::Resolving { .. }) {
            return false;
        }
        self.state = PlaybackState::Failed {
            message: TIMED_OUT.to_owned(),
        };
        true
    }

    /// Applies a resolve result and the details fetched beside it. Ignored
    /// unless `target` is the one being resolved, so a slow resolve for
    /// channel A cannot replace a later pick of channel B. Details that
    /// failed to load leave the page without them; the stream still plays.
    /// Returns whether it applied, i.e. whether the result is the one now
    /// shown.
    pub fn apply(
        &mut self,
        target: PlaybackTarget,
        result: Result<Playback, String>,
        details: Result<PlaybackDetails, String>,
    ) -> bool {
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
                    details: Box::new(details.unwrap_or_default()),
                    sleep: None,
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
            | PlaybackState::Slept { target: current }
            | PlaybackState::Ready {
                target: current, ..
            } if current != target => false,
            _ => {
                self.state = PlaybackState::Idle;
                true
            }
        }
    }

    /// Arms the sleep timer to stop playback `minutes` from `now`,
    /// replacing one already armed. Only while ready; a zero `minutes`
    /// changes nothing. Returns whether it armed. Leaving the page or a
    /// later [`Self::begin`] turns it off.
    pub fn arm_sleep(&mut self, minutes: u32, now: Instant) -> bool {
        let PlaybackState::Ready { sleep, .. } = &mut self.state else {
            return false;
        };
        let duration = Duration::from_secs(u64::from(minutes) * 60);
        match SleepTimer::arm(duration, now) {
            Some(timer) => {
                *sleep = Some(timer);
                true
            }
            None => false,
        }
    }

    /// Turns the sleep timer off; the player plays at the user's volume
    /// again. Returns whether one was armed.
    pub fn turn_off_sleep(&mut self) -> bool {
        match &mut self.state {
            PlaybackState::Ready { sleep, .. } => sleep.take().is_some(),
            _ => false,
        }
    }

    /// The armed sleep timer, `None` when there is none.
    pub fn sleep_timer(&self) -> Option<&SleepTimer> {
        match &self.state {
            PlaybackState::Ready { sleep, .. } => sleep.as_ref(),
            _ => None,
        }
    }

    /// When heads call [`Self::sleep_tick`] next; `None` while no timer
    /// is armed.
    pub fn sleep_wake(&self) -> Option<Instant> {
        self.sleep_timer().map(SleepTimer::wake)
    }

    /// Advances the sleep timer to `now`: what changed, `None` while no
    /// timer is armed. Once it expires ([`SleepStep::Expired`]) the player
    /// stops and the page shows [`PlaybackState::Slept`]. Its position
    /// needs no saving here: every report [`Self::record`] took is already
    /// saved as far as [`crate::resume_policy::SAVE_STEP`] asks, as on
    /// leaving.
    pub fn sleep_tick(&mut self, now: Instant) -> Option<SleepStep> {
        let PlaybackState::Ready {
            target,
            sleep: Some(timer),
            ..
        } = &mut self.state
        else {
            return None;
        };
        let step = timer.tick(now);
        if step == SleepStep::Expired {
            let target = target.clone();
            self.state = PlaybackState::Slept { target };
        }
        Some(step)
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
