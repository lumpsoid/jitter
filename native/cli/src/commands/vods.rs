use jitter_core::models::{FirstPage, ListState, MorePages, PageRequest, VideoKind};
use jitter_core::vods::VodsController;
use jitter_twitch::Twitch;

use super::{Result, ctx, login, twitch};

/// Prints the channel's description, when it has one, then its VODs.
pub async fn run(login_input: &str, pages: u32) -> Result<()> {
    let login = login(login_input)?;
    let twitch = twitch()?;
    let profile = twitch
        .channel_profile(&login)
        .await
        .map_err(ctx(format!("looking up {login}")))?;
    if let Some(description) = &profile.description {
        println!("{description}\n");
    }
    let mut vods = VodsController::new(login, VideoKind::Archive);
    let request = vods.begin_refresh();
    fetch(&twitch, &mut vods, request).await;
    for _ in 1..pages {
        let Some(request) = vods.begin_next() else {
            break;
        };
        fetch(&twitch, &mut vods, request).await;
    }
    let login = vods.channel();
    match vods.state() {
        ListState::Loaded { items, more, .. } => {
            for video in items {
                let seconds = video.duration.get();
                println!(
                    "{:<12} {:>3}:{:02}:{:02}  {}",
                    video.id,
                    seconds / 3600,
                    seconds / 60 % 60,
                    seconds % 60,
                    video.title
                );
            }
            match more {
                MorePages::Failed { message } => {
                    Err(format!("loading {login}'s next VODs: {message}"))
                }
                MorePages::Idle | MorePages::Loading => Ok(()),
            }
        }
        ListState::FirstPage(FirstPage::Failed { message }) => {
            Err(format!("loading {login}'s VODs: {message}"))
        }
        ListState::Idle | ListState::FirstPage(FirstPage::Loading) => {
            Err(format!("no VODs of {login} were loaded"))
        }
    }
}

async fn fetch(twitch: &Twitch, vods: &mut VodsController, request: PageRequest) {
    let result = twitch
        .channel_videos(vods.channel(), vods.kind(), request.cursor.as_ref())
        .await
        .map_err(|e| e.to_string());
    vods.apply_page(request, result);
}
