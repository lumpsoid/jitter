//! The player page: playback signals ↔ the playback and resume
//! controllers, and the player commands they lead to.

use std::fmt::Display;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use jitter_core::models::{
    self, BackgroundStep, Channel, ChannelLogin, FadeLevel, Open, Playback, Quality, QualityKind,
    QualityPreference, Seconds, SleepStep, SleepTimer, Timestamp, VideoId,
};
use jitter_core::playback::{self, PlaybackController, PlaybackState, SLEEP_PRESETS};
use jitter_core::resume::ResumeController;
use jitter_twitch::{Twitch, TwitchError};
use rinf::{RustSignal, debug_print};
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::Instant;

use crate::format::{
    sleep_left_label, sleep_preset_label, streamed_label, twitch_error, uptime_label, viewers_label,
};
use crate::run::Fetched;
use crate::signals::{
    AppBackgrounded, AppForegrounded, ArmSleepTimer, LeavePlayer, PlaybackChannel, PlaybackDetails,
    PlaybackLive, PlaybackSleep, PlaybackTarget, PlaybackUpdate, PlayerCommand, PlayerPosition,
    QualityOption, SelectQuality, SleepPreset,
};

/// `Play`: starts resolving `target` through twitch, and loading what it
/// plays beside it; both come back together as [`Fetched::Playback`], or
/// as [`Fetched::PlaybackTimedOut`] when they take longer than core's
/// `RESOLVE_TIMEOUT`. A target core rejects fails at once. Returns the
/// target now playing, `None` when it was rejected.
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
    let attempt = playback.begin(target.clone());
    send_state(playback);
    let (twitch, fetch, playing) = (twitch.clone(), fetch.clone(), target.clone());
    tokio::spawn(async move {
        let resolve = async {
            match &target {
                playback::PlaybackTarget::Live(login) => twitch.live(login).await,
                playback::PlaybackTarget::Vod(id) => twitch.vod(id).await,
            }
        };
        let both = async { tokio::join!(resolve, twitch.playback_details(&target)) };
        let limit = Duration::from_secs(u64::from(playback::RESOLVE_TIMEOUT.get()));
        let _ = fetch.send(match tokio::time::timeout(limit, both).await {
            Ok((result, details)) => Fetched::Playback {
                target,
                result,
                details,
            },
            Err(_) => Fetched::PlaybackTimedOut(attempt),
        });
    });
    Some(playing)
}

/// A resolve ran out of time: core decides whether the page fails.
pub fn timed_out(playback: &mut PlaybackController, attempt: playback::Attempt) {
    if playback.time_out(attempt) {
        send_state(playback);
    }
}

/// A resolve finished: shows the result and, if it is the one being
/// resolved, opens the quality core picks for `preferred` (the user's
/// start quality) where core says to start. A stale result changes
/// nothing. A details failure is logged; core decides what the page shows
/// without them.
pub fn resolved(
    playback: &mut PlaybackController,
    resume: &ResumeController,
    target: playback::PlaybackTarget,
    result: Result<Playback, TwitchError>,
    details: Result<models::PlaybackDetails, TwitchError>,
    preferred: QualityPreference,
) {
    let details = details
        .map_err(|e| e.to_string())
        .inspect_err(|e| debug_print!("loading the playback details: {e}"));
    if !playback.apply(
        target,
        result.map_err(|e| twitch_error(&e)),
        details,
        preferred,
    ) {
        return;
    }
    match playback.open(resume) {
        Ok(open) => {
            send_state(playback);
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
    send_state(playback);
    send_open(open);
}

/// `AppBackgrounded`: the player carries on with what core picks for
/// `audio` (the user's background audio setting), or stops.
pub fn background(playback: &mut PlaybackController, audio: bool, signal: AppBackgrounded) {
    match playback.background(audio, Seconds::new(signal.position_s)) {
        None => {}
        Some(BackgroundStep::Open(open)) => send_open(Some(open)),
        Some(BackgroundStep::Stop) => PlayerCommand::Stop.send_signal_to_dart(),
    }
}

/// `AppForegrounded`: the player carries on with the selected quality.
pub fn foreground(
    playback: &mut PlaybackController,
    resume: &ResumeController,
    signal: AppForegrounded,
) {
    match playback.foreground(Seconds::new(signal.position_s), resume) {
        Ok(open) => send_open(open),
        Err(e) => PlaybackUpdate::Failed {
            message: format!("Could not read the resume point: {e}"),
        }
        .send_signal_to_dart(),
    }
}

/// Where a player is, as core's values: what a `PlayerPosition` reports.
pub struct PlayerAt {
    pub target: playback::PlaybackTarget,
    pub position: Seconds,
    pub duration: Seconds,
}

/// `PlayerPosition`: hands the position to core, which decides whether
/// it is recorded. Returns the report for the chat replay; `None` when
/// its target is invalid or core says the player is stopped away, where
/// its position means nothing.
pub fn position(
    playback: &PlaybackController,
    resume: &mut ResumeController,
    signal: PlayerPosition,
) -> Option<PlayerAt> {
    let target = report(playback::PlaybackTarget::try_from(signal.target))?;
    if playback.stopped_away() {
        return None;
    }
    report(record(
        playback,
        resume,
        &target,
        signal.position_s,
        signal.duration_s,
    ));
    Some(PlayerAt {
        target,
        position: Seconds::new(signal.position_s),
        duration: Seconds::new(signal.duration_s),
    })
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
        send_state(playback);
        PlayerCommand::Stop.send_signal_to_dart();
    }
    left
}

/// `ArmSleepTimer`: core arms the page's timer as of now; its wakes then
/// come through [`sleep_wake`].
pub fn arm_sleep(playback: &mut PlaybackController, signal: ArmSleepTimer) {
    if playback.arm_sleep(signal.minutes, Instant::now().into_std()) {
        send_state(playback);
    }
}

/// `TurnOffSleepTimer`: the timer is gone and the player is back at the
/// user's volume, even mid-fade.
pub fn turn_off_sleep(playback: &mut PlaybackController) {
    if playback.turn_off_sleep() {
        send_state(playback);
        send_fade(FadeLevel::FULL);
    }
}

/// When the sleep timer next needs a tick; `None` while none is armed, so
/// a turned-off or replaced timer is never ticked. Read afresh each loop.
pub fn sleep_wake(playback: &PlaybackController) -> Option<Instant> {
    playback.sleep_wake().map(Instant::from_std)
}

/// The sleep timer's wake: sends what core says changed. When it expired
/// the player stops; returns whether it did, so the chat closes too.
pub fn sleep_tick(playback: &mut PlaybackController) -> bool {
    match playback.sleep_tick(Instant::now().into_std()) {
        None => false,
        Some(SleepStep::Counting { relabeled, fade }) => {
            if relabeled {
                send_state(playback);
            }
            if let Some(level) = fade {
                send_fade(level);
            }
            false
        }
        Some(SleepStep::Expired) => {
            send_state(playback);
            PlayerCommand::Stop.send_signal_to_dart();
            true
        }
    }
}

fn send_fade(level: FadeLevel) {
    PlayerCommand::SetFade { level: level.get() }.send_signal_to_dart();
}

/// Sends the page's state, a live stream's uptime as of now. A clock that
/// cannot be read is logged, and the state goes out without the uptime.
fn send_state(playback: &PlaybackController) {
    update(playback.state(), report(now())).send_signal_to_dart();
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

/// The page's state as sent, a live stream's uptime as of `now` (none
/// without a clock).
fn update(state: &PlaybackState, now: Option<Timestamp>) -> PlaybackUpdate {
    match state {
        PlaybackState::Idle => PlaybackUpdate::Idle,
        PlaybackState::Resolving { .. } => PlaybackUpdate::Resolving,
        PlaybackState::Ready {
            target,
            playback,
            selected,
            details,
            sleep,
        } => PlaybackUpdate::Ready {
            qualities: playback.qualities.iter().map(QualityOption::from).collect(),
            selected: selected.clone(),
            live: matches!(target, playback::PlaybackTarget::Live(_)),
            details: Box::new(playback_details(details, now)),
            sleep: playback_sleep(sleep.as_ref()),
        },
        PlaybackState::Slept { .. } => PlaybackUpdate::Slept,
        PlaybackState::Failed { message } => PlaybackUpdate::Failed {
            message: message.clone(),
        },
    }
}

fn playback_sleep(timer: Option<&SleepTimer>) -> PlaybackSleep {
    PlaybackSleep {
        presets: SLEEP_PRESETS
            .iter()
            .map(|&minutes| SleepPreset {
                minutes,
                label: sleep_preset_label(minutes),
            })
            .collect(),
        left: timer.map(|timer| sleep_left_label(timer.minutes_left())),
    }
}

fn playback_details(details: &models::PlaybackDetails, now: Option<Timestamp>) -> PlaybackDetails {
    PlaybackDetails {
        channel: details.channel.as_ref().map(PlaybackChannel::from),
        game: details.game.as_ref().map(|game| game.name.clone()),
        streamed: details.streamed_at.and_then(streamed_label),
        live: details.live.map(|live| PlaybackLive {
            viewers: viewers_label(live.viewers),
            uptime: now.and_then(|now| live.uptime(now)).map(uptime_label),
        }),
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
        let details = playback_details(
            &models::PlaybackDetails {
                channel: None,
                game: Some(GameRef {
                    id,
                    name: "Just Chatting".to_owned(),
                }),
                streamed_at: Some(at),
                live: None,
            },
            None,
        );
        assert_eq!(details.game.as_deref(), Some("Just Chatting"));
        assert_eq!(details.streamed, streamed_label(at));
        assert!(details.streamed.is_some());
        Ok(())
    }

    #[test]
    fn details_without_a_date_send_none() {
        let details = playback_details(
            &models::PlaybackDetails {
                channel: None,
                game: None,
                streamed_at: None,
                live: None,
            },
            None,
        );
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
        let details = playback_details(
            &models::PlaybackDetails {
                channel: Some(channel("https://example.test/xqc.jpeg")?),
                game: None,
                streamed_at: None,
                live: None,
            },
            None,
        );
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

    fn live(started_at: Option<i64>) -> models::PlaybackDetails {
        models::PlaybackDetails {
            live: Some(models::LiveDetails {
                viewers: 1_234,
                started_at: started_at.map(Timestamp::from_unix),
            }),
            ..Default::default()
        }
    }

    #[test]
    fn a_live_stream_sends_its_viewers_and_uptime_as_of_now() -> Result<(), String> {
        let now = Some(Timestamp::from_unix(1_000 + 8_100));
        let sent = playback_details(&live(Some(1_000)), now)
            .live
            .ok_or("no live details")?;
        assert_eq!(sent.viewers, "1.2K viewers");
        assert_eq!(sent.uptime.as_deref(), Some("2h 15m"));
        Ok(())
    }

    #[test]
    fn no_start_time_or_no_clock_means_no_uptime() -> Result<(), String> {
        let now = Some(Timestamp::from_unix(100));
        let unstarted = playback_details(&live(None), now).live;
        assert_eq!(unstarted.ok_or("no live details")?.uptime, None);
        let clockless = playback_details(&live(Some(0)), None).live;
        let clockless = clockless.ok_or("no live details")?;
        assert_eq!(clockless.viewers, "1.2K viewers");
        assert_eq!(clockless.uptime, None);
        Ok(())
    }

    #[test]
    fn a_vod_sends_no_live_details() {
        let now = Some(Timestamp::from_unix(100));
        let details = playback_details(&models::PlaybackDetails::default(), now);
        assert!(details.live.is_none());
    }
}
