//! The player page: playback signals ↔ the playback and resume
//! controllers, and the player commands they lead to.

use std::fmt::Display;
use std::time::{SystemTime, UNIX_EPOCH};

use jitter_core::models::{
    ChannelLogin, Open, Playback, Quality, QualityKind, Seconds, Timestamp, VideoId,
};
use jitter_core::playback::{self, PlaybackController, PlaybackState};
use jitter_core::resume::ResumeController;
use jitter_twitch::{Twitch, TwitchError};
use rinf::{RustSignal, debug_print};
use tokio::sync::mpsc::UnboundedSender;

use crate::format::twitch_error;
use crate::run::Fetched;
use crate::signals::{
    LeavePlayer, PlaybackTarget, PlaybackUpdate, PlayerCommand, PlayerPosition, QualityOption,
    SelectQuality,
};

/// `Play`: starts resolving `target` through twitch; the result comes back
/// as [`Fetched::Playback`]. A target core rejects fails at once. Returns
/// the target now playing, `None` when it was rejected.
pub fn play(
    playback: &mut PlaybackController,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
    target: PlaybackTarget,
) -> Option<playback::PlaybackTarget> {
    let target = match playback::PlaybackTarget::try_from(target) {
        Ok(target) => target,
        Err(message) => {
            PlaybackUpdate::Failed { message }.send_signal_to_dart();
            return None;
        }
    };
    playback.begin(target.clone());
    PlaybackUpdate::from(playback.state()).send_signal_to_dart();
    let (twitch, fetch, playing) = (twitch.clone(), fetch.clone(), target.clone());
    tokio::spawn(async move {
        let result = match &target {
            playback::PlaybackTarget::Live(login) => twitch.live(login).await,
            playback::PlaybackTarget::Vod(id) => twitch.vod(id).await,
        };
        let _ = fetch.send(Fetched::Playback { target, result });
    });
    Some(playing)
}

/// A resolve finished: shows the result and, if it is the one being
/// resolved, opens the selected quality where core says to start. A stale
/// result changes nothing.
pub fn resolved(
    playback: &mut PlaybackController,
    resume: &ResumeController,
    target: playback::PlaybackTarget,
    result: Result<Playback, TwitchError>,
) {
    if !playback.apply(target, result.map_err(|e| twitch_error(&e))) {
        return;
    }
    match playback.open(resume) {
        Ok(open) => {
            PlaybackUpdate::from(playback.state()).send_signal_to_dart();
            send_open(open);
        }
        Err(e) => PlaybackUpdate::Failed {
            message: format!("Could not read the resume point: {e}"),
        }
        .send_signal_to_dart(),
    }
}

/// `SelectQuality`: switches mid-play, keeping the position; an unknown
/// name changes nothing.
pub fn select_quality(playback: &mut PlaybackController, signal: SelectQuality) {
    let open = playback.switch_quality(&signal.name, Seconds::new(signal.position_s));
    PlaybackUpdate::from(playback.state()).send_signal_to_dart();
    send_open(open);
}

/// `PlayerPosition`: hands the position to core, which decides whether
/// it is recorded.
pub fn position(
    playback: &PlaybackController,
    resume: &mut ResumeController,
    signal: PlayerPosition,
) {
    let Some(target) = report(playback::PlaybackTarget::try_from(signal.target)) else {
        return;
    };
    report(record(
        playback,
        resume,
        &target,
        signal.position_s,
        signal.duration_s,
    ));
}

/// `LeavePlayer`: records the last position, then leaves; the player stops
/// unless a later page already took it over. Returns whether it stopped.
pub fn leave(
    playback: &mut PlaybackController,
    resume: &mut ResumeController,
    signal: LeavePlayer,
) -> bool {
    let Some(target) = report(playback::PlaybackTarget::try_from(signal.target)) else {
        return false;
    };
    report(record(
        playback,
        resume,
        &target,
        signal.position_s,
        signal.duration_s,
    ));
    let left = playback.leave(&target);
    if left {
        PlaybackUpdate::from(playback.state()).send_signal_to_dart();
        PlayerCommand::Stop.send_signal_to_dart();
    }
    left
}

fn send_open(open: Option<Open>) {
    if let Some(open) = open {
        PlayerCommand::from(open).send_signal_to_dart();
    }
}

fn record(
    playback: &PlaybackController,
    resume: &mut ResumeController,
    target: &playback::PlaybackTarget,
    position_s: u32,
    duration_s: u32,
) -> Result<(), String> {
    playback
        .record(
            resume,
            target,
            Seconds::new(position_s),
            Seconds::new(duration_s),
            now()?,
        )
        .map_err(|e| format!("saving the resume point: {e}"))?;
    Ok(())
}

/// Logs a failure no view shows: the player page keeps playing.
fn report<T>(result: Result<T, impl Display>) -> Option<T> {
    result.map_err(|error| debug_print!("{error}")).ok()
}

/// Heads pass the clock to core; core never reads it.
fn now() -> Result<Timestamp, String> {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("reading the clock: {e}"))?;
    let seconds =
        i64::try_from(since_epoch.as_secs()).map_err(|e| format!("reading the clock: {e}"))?;
    Ok(Timestamp::from_unix(seconds))
}

impl TryFrom<PlaybackTarget> for playback::PlaybackTarget {
    type Error = String;

    fn try_from(target: PlaybackTarget) -> Result<Self, String> {
        match target {
            PlaybackTarget::Live { login } => ChannelLogin::parse(&login)
                .map(Self::Live)
                .map_err(|e| format!("invalid channel login {login:?}: {e:?}")),
            PlaybackTarget::Vod { id } => VideoId::parse(&id)
                .map(Self::Vod)
                .map_err(|e| format!("invalid video id {id:?}: {e:?}")),
        }
    }
}

impl From<&PlaybackState> for PlaybackUpdate {
    fn from(state: &PlaybackState) -> Self {
        match state {
            PlaybackState::Idle => Self::Idle,
            PlaybackState::Resolving { .. } => Self::Resolving,
            PlaybackState::Ready {
                target,
                playback,
                selected,
            } => Self::Ready {
                qualities: playback.qualities.iter().map(QualityOption::from).collect(),
                selected: selected.clone(),
                live: matches!(target, playback::PlaybackTarget::Live(_)),
            },
            PlaybackState::Failed { message } => Self::Failed {
                message: message.clone(),
            },
        }
    }
}

impl From<&Quality> for QualityOption {
    fn from(quality: &Quality) -> Self {
        Self {
            name: quality.name.clone(),
            label: match quality.kind {
                QualityKind::Source => format!("{} (source)", quality.name),
                QualityKind::Transcode => quality.name.clone(),
                QualityKind::AudioOnly => "Audio only".to_owned(),
            },
        }
    }
}

impl From<Open> for PlayerCommand {
    fn from(open: Open) -> Self {
        Self::Open {
            url: open.url,
            start_s: open.start.get(),
        }
    }
}
