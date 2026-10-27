use jitter_core::category::{CategoryController, pin_favorites};
use jitter_core::models::{CategoryId, FirstPage, ListState, MorePages, PageRequest, Stream};
use jitter_twitch::Twitch;

use super::{Result, live_favorites, twitch};

/// One category's live streams, your favorites playing it first (marked
/// `*`).
pub async fn run(id: &str, pages: u32) -> Result<()> {
    let id = CategoryId::parse(id).map_err(|e| format!("invalid category id {id:?}: {e:?}"))?;
    let twitch = twitch()?;
    let favorites = live_favorites(&twitch).await?;

    let mut category = CategoryController::new(id.clone());
    let request = category.begin_refresh();
    fetch(&twitch, &mut category, request).await;
    for _ in 1..pages {
        let Some(request) = category.begin_next() else {
            break;
        };
        fetch(&twitch, &mut category, request).await;
    }
    match category.state() {
        ListState::Loaded { items, more, .. } => {
            let pinned = pin_favorites(&id, favorites.entries(), items);
            for stream in &pinned.favorites {
                print_stream('*', stream);
            }
            for stream in &pinned.others {
                print_stream(' ', stream);
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

fn print_stream(mark: char, stream: &Stream) {
    println!(
        "{mark} {:>8}  {:<20} {}",
        stream.viewers, stream.channel.login, stream.title
    );
}

async fn fetch(twitch: &Twitch, category: &mut CategoryController, request: PageRequest) {
    let result = twitch
        .category_streams(category.id(), request.cursor.as_ref())
        .await
        .map_err(|e| e.to_string());
    category.apply_page(request, result);
}
