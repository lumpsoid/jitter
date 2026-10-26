use clap::Args;
use jitter_core::playback::{PlaybackController, PlaybackState, PlaybackTarget};
use jitter_core::resume::ResumeController;

use super::{Result, ctx, login, open_store, twitch, video_id};

/// The quality name that means "whatever core selects by default".
const DEFAULT_QUALITY: &str = "source";

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
    #[arg(long, value_name = "QUALITY", num_args = 0..=1, default_missing_value = DEFAULT_QUALITY, conflicts_with = "start")]
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

    let twitch = twitch()?;
    let mut playback = PlaybackController::new();
    playback.begin(target.clone());
    let resolved = match &target {
        PlaybackTarget::Live(login) => twitch.live(login).await,
        PlaybackTarget::Vod(id) => twitch.vod(id).await,
    };
    playback.apply(target, resolved.map_err(|e| e.to_string()));

    if let Some(name) = &args.url {
        playback.select(name);
    }
    match playback.state() {
        PlaybackState::Ready {
            playback: resolved,
            selected,
            ..
        } => match &args.url {
            Some(name) => {
                let quality = playback
                    .selected_quality()
                    .filter(|q| name == DEFAULT_QUALITY || q.name == *name)
                    .ok_or_else(|| format!("no quality named {name:?}"))?;
                println!("{}", quality.url);
            }
            None => {
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
            }
        },
        PlaybackState::Failed { message } => return Err(message.clone()),
        PlaybackState::Idle | PlaybackState::Resolving { .. } => {
            return Err("playback did not resolve".to_owned());
        }
    }
    Ok(())
}
