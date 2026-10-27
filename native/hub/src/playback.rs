//! The player page: playback signals ↔ the playback and resume
//! controllers, and the player commands they lead to.

use std::fmt::Display;
use std::time::{SystemTime, UNIX_EPOCH};

use jitter_core::models::{
    self, Channel, ChannelLogin, Open, Playback, Quality, QualityKind, Seconds, Timestamp, VideoId,
};
use jitter_core::playback::{self, PlaybackController, PlaybackState};
use jitter_core::resume::ResumeController;
use jitter_twitch::{Twitch, TwitchError};
use rinf::{RustSignal, debug_print};
use tokio::sync::mpsc::UnboundedSender;

use crate::format::{streamed_label, twitch_error};
use crate::run::Fetched;
use crate::signals::{
    LeavePlayer, PlaybackChannel, PlaybackDetails, PlaybackTarget, PlaybackUpdate, PlayerCommand,
    PlayerPosition, QualityOption, SelectQuality,
};

/// `Play`: starts resolving `target` through twitch, and loading what it
/// plays beside it; both come back together as [`Fetched::Playback`]. A
/// target core rejects fails at once. Returns the target now playing,
/// `None` when it was rejected.
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
        let resolve = async {
            match &target {
                playback::PlaybackTarget::Live(login) => twitch.live(login).await,
                playback::PlaybackTarget::Vod(id) => twitch.vod(id).await,
            }
        };
        let (result, details) = tokio::join!(resolve, twitch.playback_details(&target));
        let _ = fetch.send(Fetched::Playback {
            target,
            result,
            details,
        });
    });
    Some(playing)
}

/// A resolve finished: shows the result and, if it is the one being
/// resolved, opens the selected quality where core says to start. A stale
/// result changes nothing. A details failure is logged; core decides what
/// the page shows without them.
pub fn resolved(
    playback: &mut PlaybackController,
    resume: &ResumeController,
    target: playback::PlaybackTarget,
    result: Result<Playback, TwitchError>,
    details: Result<models::PlaybackDetails, TwitchError>,
) {
    let details = details
        .map_err(|e| e.to_string())
        .inspect_err(|e| debug_print!("loading the playback details: {e}"));
    if !playback.apply(target, result.map_err(|e| twitch_error(&e)), details) {
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
                details,
            } => Self::Ready {
                qualities: playback.qualities.iter().map(QualityOption::from).collect(),
                selected: selected.clone(),
                live: matches!(target, playback::PlaybackTarget::Live(_)),
                details: PlaybackDetails::from(details.as_ref()),
            },
            PlaybackState::Failed { message } => Self::Failed {
                message: message.clone(),
            },
        }
    }
}

impl From<&models::PlaybackDetails> for PlaybackDetails {
    fn from(details: &models::PlaybackDetails) -> Self {
        Self {
            channel: details.channel.as_ref().map(PlaybackChannel::from),
            game: details.game.as_ref().map(|game| game.name.clone()),
            streamed: details.streamed_at.and_then(streamed_label),
        }
    }
}

impl From<&Channel> for PlaybackChannel {
    fn from(channel: &Channel) -> Self {
        Self {
            login: channel.login.as_str().to_owned(),
            display_name: channel.display_name.clone(),
            avatar_url: channel.avatar().map(str::to_owned),
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

#[cfg(test)]
mod tests {
    use jitter_core::models::{CategoryId, ChannelId, GameRef};

    use super::*;

    #[test]
    fn details_send_the_game_name_and_the_streamed_day() -> Result<(), String> {
        let id = CategoryId::parse("509658").map_err(|e| format!("{e:?}"))?;
        let at = Timestamp::from_unix(1_792_875_000);
        let details = PlaybackDetails::from(&models::PlaybackDetails {
            channel: None,
            game: Some(GameRef {
                id,
                name: "Just Chatting".to_owned(),
            }),
            streamed_at: Some(at),
        });
        assert_eq!(details.game.as_deref(), Some("Just Chatting"));
        assert_eq!(details.streamed, streamed_label(at));
        assert!(details.streamed.is_some());
        Ok(())
    }

    #[test]
    fn details_without_a_date_send_none() {
        let details = PlaybackDetails::from(&models::PlaybackDetails {
            channel: None,
            game: None,
            streamed_at: None,
        });
        assert!(details.channel.is_none());
        assert_eq!(details.game, None);
        assert_eq!(details.streamed, None);
    }

    #[test]
    fn details_send_the_channel_by_login_with_its_avatar() -> Result<(), String> {
        let channel = |avatar_url: &str| -> Result<Channel, String> {
            Ok(Channel {
                id: ChannelId::parse("71092938").map_err(|e| format!("{e:?}"))?,
                login: ChannelLogin::parse("xqc").map_err(|e| format!("{e:?}"))?,
                display_name: "xQc".to_owned(),
                avatar_url: avatar_url.to_owned(),
            })
        };
        let details = PlaybackDetails::from(&models::PlaybackDetails {
            channel: Some(channel("https://example.test/xqc.jpeg")?),
            game: None,
            streamed_at: None,
        });
        let sent = details.channel.ok_or("no channel")?;
        assert_eq!(sent.login, "xqc");
        assert_eq!(sent.display_name, "xQc");
        assert_eq!(
            sent.avatar_url.as_deref(),
            Some("https://example.test/xqc.jpeg")
        );
        assert_eq!(PlaybackChannel::from(&channel("")?).avatar_url, None);
        Ok(())
    }
}
