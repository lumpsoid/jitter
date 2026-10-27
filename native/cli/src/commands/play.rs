use std::time::Duration;

use chrono::{DateTime, Local, TimeZone};
use clap::Args;
use jitter_core::models::{Playback, Timestamp};
use jitter_core::playback::{PlaybackController, PlaybackState, PlaybackTarget, RESOLVE_TIMEOUT};
use jitter_core::resume::ResumeController;

use super::{Result, ctx, login, now, open_store, settings, twitch, video_id};

#[derive(Args)]
pub struct PlayArgs {
    /// Channel login to watch live.
    #[arg(required_unless_present = "vod", conflicts_with = "vod")]
    login: Option<String>,
    /// Watch a VOD instead.
    #[arg(long)]
    vod: Option<String>,
    /// Print only one URL: the named quality (for example `720p60`), or the
    /// default (`source`) when the name is omitted or is `source`.
    #[arg(long, value_name = "QUALITY", num_args = 0..=1, default_missing_value = Playback::DEFAULT_QUALITY, conflicts_with = "start")]
    url: Option<String>,
    /// Print the resume offset of the VOD in seconds; no network.
    #[arg(long)]
    start: bool,
}

pub async fn run(args: PlayArgs) -> Result<()> {
    let target = match (&args.login, &args.vod) {
        (_, Some(id)) => PlaybackTarget::Vod(video_id(id)?),
        (Some(input), None) => PlaybackTarget::Live(login(input)?),
        (None, None) => return Err("give a channel login or --vod <id>".to_owned()),
    };
    if args.start {
        let PlaybackTarget::Vod(id) = &target else {
            return Err("--start applies to --vod only".to_owned());
        };
        let resume = ResumeController::new(Box::new(open_store()?));
        let start = resume
            .start_position(id)
            .map_err(ctx("reading the resume point"))?;
        println!("{}", start.get());
        return Ok(());
    }

    let preferred = settings()?.start_quality;
    let twitch = twitch()?;
    let mut playback = PlaybackController::new();
    let attempt = playback.begin(target.clone());
    let resolve = async {
        match &target {
            PlaybackTarget::Live(login) => twitch.live(login).await,
            PlaybackTarget::Vod(id) => twitch.vod(id).await,
        }
    };
    let both = async { tokio::join!(resolve, twitch.playback_details(&target)) };
    let limit = Duration::from_secs(u64::from(RESOLVE_TIMEOUT.get()));
    match tokio::time::timeout(limit, both).await {
        Ok((resolved, details)) => {
            if let Err(e) = &details {
                eprintln!("-- playback details unavailable: {e}");
            }
            playback.apply(
                target,
                resolved.map_err(|e| e.to_string()),
                details.map_err(|e| e.to_string()),
                preferred,
            );
        }
        Err(_) => {
            playback.time_out(attempt);
        }
    }

    let not_ready = || "playback did not resolve".to_owned();
    match playback.state() {
        PlaybackState::Ready { .. } => {}
        PlaybackState::Failed { message } => return Err(message.clone()),
        PlaybackState::Idle | PlaybackState::Resolving { .. } | PlaybackState::Slept { .. } => {
            return Err(not_ready());
        }
    }
    if let Some(name) = &args.url {
        if !playback.select(name) {
            return Err(format!("no quality named {name:?}"));
        }
        let resume = ResumeController::new(Box::new(open_store()?));
        let open = playback
            .open(&resume)
            .map_err(ctx("reading the resume point"))?
            .ok_or_else(not_ready)?;
        println!("{}", open.url);
        return Ok(());
    }
    let PlaybackState::Ready {
        playback: resolved,
        selected,
        details,
        ..
    } = playback.state()
    else {
        return Err(not_ready());
    };
    if let Some(channel) = &details.channel {
        println!(
            "Channel: {} ({})",
            channel.display_name,
            channel.login.as_str()
        );
    }
    if let Some(game) = &details.game {
        println!("Game: {}", game.name);
    }
    if let Some(streamed) = details.streamed_at.and_then(local_time) {
        println!("Streamed: {}", streamed.format("%Y-%m-%d %H:%M %:z"));
    }
    if let Some(live) = &details.live {
        let uptime = live.uptime(now()?).map_or_else(
            || "?".to_owned(),
            |up| format!("{}h{:02}m", up.get() / 3600, up.get() / 60 % 60),
        );
        println!("Live: {} viewers, up {uptime}", live.viewers);
    }
    for q in &resolved.qualities {
        let marker = if q.name == *selected { '*' } else { ' ' };
        let size = q
            .resolution
            .map_or_else(String::new, |r| format!("{}x{}", r.width, r.height));
        let rate = q.frame_rate.map_or_else(String::new, |f| format!("{f}fps"));
        println!(
            "{marker} {:<12} {:<10} {:<6} {:?}",
            q.name, size, rate, q.kind
        );
    }
    Ok(())
}

/// `at` in the machine's time zone; `None` outside chrono's range.
fn local_time(at: Timestamp) -> Option<DateTime<Local>> {
    Local.timestamp_opt(at.unix(), 0).single()
}
