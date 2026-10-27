use jitter_core::categories::{TopCategoriesController, favorites_by_category};
use jitter_core::models::{FirstPage, ListState, MorePages, PageRequest};
use jitter_twitch::Twitch;

use super::{Result, live_favorites, twitch};

/// The categories your live favorites are streaming, then the top
/// categories by viewers.
pub async fn run(pages: u32) -> Result<()> {
    let twitch = twitch()?;
    let favorites = live_favorites(&twitch).await?;
    let groups = favorites_by_category(favorites.entries());
    if !groups.is_empty() {
        println!("From your channels:");
        for group in &groups {
            let channels: Vec<&str> = group
                .streams
                .iter()
                .map(|s| s.channel.display_name.as_str())
                .collect();
            println!(
                "{:>10}  {:<32} {}",
                group.game.id,
                group.game.name,
                channels.join(", ")
            );
        }
        println!();
    }

    let mut top = TopCategoriesController::new();
    let request = top.begin_refresh();
    fetch(&twitch, &mut top, request).await;
    for _ in 1..pages {
        let Some(request) = top.begin_next() else {
            break;
        };
        fetch(&twitch, &mut top, request).await;
    }
    match top.state() {
        ListState::Loaded { items, more, .. } => {
            for category in items {
                println!(
                    "{:>10}  {:<32} {:>8}",
                    category.id, category.name, category.viewers
                );
            }
            match more {
                MorePages::Failed { message } => Err(format!("loading the next page: {message}")),
                MorePages::Idle | MorePages::Loading => Ok(()),
            }
        }
        ListState::FirstPage(FirstPage::Failed { message }) => Err(message.clone()),
        ListState::Idle | ListState::FirstPage(FirstPage::Loading) => {
            Err("no categories were loaded".to_owned())
        }
    }
}

async fn fetch(twitch: &Twitch, top: &mut TopCategoriesController, request: PageRequest) {
    let result = twitch
        .top_categories(request.cursor.as_ref())
        .await
        .map_err(|e| e.to_string());
    top.apply_page(request, result);
}
