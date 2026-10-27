use jitter_core::chat_replay::{ChatReplay, ReplayRequest, ReplayStep};
use jitter_core::models::{ChatChange, EmoteProvider, EmoteScope, Seconds};
use jitter_core::playback::PlaybackTarget;
use jitter_twitch::Twitch;
use jitter_twitch::emotes::ProviderEmotes;

use super::chat::render;
use super::{Result, settings, twitch, video_id};

/// Replays a VOD's chat as a player would see it from `at` for `span`
/// seconds, without waiting: the position is reported a second at a time
/// and each page core asks for is fetched before the next report. Each
/// message is printed with the video position it appeared at.
pub async fn run(input: &str, at: u32, span: u32) -> Result<()> {
    let id = video_id(input)?;
    let twitch = twitch()?;
    let target = PlaybackTarget::Vod(id.clone());
    let mut replay = ChatReplay::new(id);
    let providers = settings()?.emote_providers();
    load_emotes(&twitch, &target, providers, &mut replay).await;

    // Any non-zero duration: core only needs to know the player loaded.
    let duration = Seconds::new(at.saturating_add(span).saturating_add(1));
    for second in at..=at.saturating_add(span) {
        let position = Seconds::new(second);
        let mut step = replay.position(&target, position, duration);
        loop {
            print_changes(&replay, position, &step);
            let Some(request) = step.fetch.take() else {
                break;
            };
            step = fetch(&twitch, &mut replay, request).await;
        }
        if let Some(failure) = replay.failure() {
            return Err(format!("loading the chat: {failure}"));
        }
    }
    Ok(())
}

async fn fetch(twitch: &Twitch, replay: &mut ChatReplay, request: ReplayRequest) -> ReplayStep {
    let result = twitch
        .video_comments(&request.video, &request.from)
        .await
        .map_err(|e| e.to_string());
    replay.apply(request, result)
}

fn print_changes(replay: &ChatReplay, position: Seconds, step: &ReplayStep) {
    for change in &step.changes {
        match change {
            ChatChange::Appended { message } => {
                println!("[{}] {}", clock(position), render(replay.chat(), message));
            }
            ChatChange::Cleared => println!("-- chat cleared (seek)"),
            _ => {}
        }
    }
}

/// `1:02:03`.
fn clock(position: Seconds) -> String {
    let s = position.get();
    format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

/// The VOD's channel emotes need its owner's id; if it cannot be found
/// only the global ones load. Only `providers` are asked.
async fn load_emotes(
    twitch: &Twitch,
    target: &PlaybackTarget,
    providers: &[EmoteProvider],
    replay: &mut ChatReplay,
) {
    let owner = match twitch.playback_details(target).await {
        Ok(details) => details.channel,
        Err(e) => {
            eprintln!("-- looking up the VOD's channel: {e}");
            None
        }
    };
    let global = twitch.global_emotes(providers).await;
    set(replay, EmoteScope::Global, global);
    if let Some(channel) = owner {
        let local = twitch.channel_emotes(&channel.id, providers).await;
        set(replay, EmoteScope::Channel, local);
    }
}

fn set(replay: &mut ChatReplay, scope: EmoteScope, results: Vec<ProviderEmotes>) {
    for (provider, result) in results {
        match result {
            Ok(emotes) => {
                replay.set_emotes(scope, provider, emotes);
            }
            Err(e) => {
                eprintln!("-- {provider:?} {scope:?} emotes unavailable: {e}");
                replay.provider_failed(provider);
            }
        }
    }
}
