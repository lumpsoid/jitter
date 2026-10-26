use jitter_core::browse::{BrowseController, FirstPage, ListState, MorePages, PageRequest};
use jitter_twitch::Twitch;

use super::{Result, twitch};

pub async fn run(pages: u32) -> Result<()> {
    let twitch = twitch()?;
    let mut browse = BrowseController::new();
    let request = browse.begin_refresh();
    fetch(&twitch, &mut browse, request).await;
    for _ in 1..pages {
        let Some(request) = browse.begin_next() else {
            break;
        };
        fetch(&twitch, &mut browse, request).await;
    }
    match browse.state() {
        ListState::Loaded { items, more, .. } => {
            for stream in items {
                println!(
                    "{:>8}  {:<20} {:<24} {}",
                    stream.viewers,
                    stream.channel.login,
                    stream.game_name.as_deref().unwrap_or("-"),
                    stream.title
                );
            }
            match more {
                MorePages::Failed { message } => Err(format!("loading the next page: {message}")),
                MorePages::Idle | MorePages::Loading => Ok(()),
            }
        }
        ListState::FirstPage(FirstPage::Failed { message }) => Err(message.clone()),
        ListState::Idle | ListState::FirstPage(FirstPage::Loading) => {
            Err("no streams were loaded".to_owned())
        }
    }
}

async fn fetch(twitch: &Twitch, browse: &mut BrowseController, request: PageRequest) {
    let result = twitch
        .top_streams(request.cursor.as_ref())
        .await
        .map_err(|e| e.to_string());
    browse.apply_page(request, result);
}
