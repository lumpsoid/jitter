use jitter_core::models::{
    Category, FirstPage, FoundChannel, ListState, MorePages, PageRequest, SearchKind, SearchQuery,
};
use jitter_core::search::SearchController;
use jitter_twitch::Twitch;

use super::{Result, twitch};

/// The channels (login, name, followers, live viewers), then the
/// categories (id, name, viewers), matching `text`.
pub async fn run(text: &str, pages: u32) -> Result<()> {
    let query = SearchQuery::parse(text).map_err(|e| format!("invalid search {text:?}: {e:?}"))?;
    let twitch = twitch()?;

    let mut search = SearchController::new();
    let requests = search
        .search(query)
        .ok_or_else(|| "nothing to search".to_owned())?;
    fetch_channels(&twitch, &mut search, requests.channels).await;
    fetch_categories(&twitch, &mut search, requests.categories).await;
    for _ in 1..pages {
        if let Some(request) = search.begin_next(SearchKind::Channels) {
            fetch_channels(&twitch, &mut search, request).await;
        }
        if let Some(request) = search.begin_next(SearchKind::Categories) {
            fetch_categories(&twitch, &mut search, request).await;
        }
    }

    println!("Channels:");
    let channels = print_list(search.channels(), print_channel);
    println!();
    println!("Categories:");
    let categories = print_list(search.categories(), print_category);
    channels.map_err(|e| format!("channels: {e}"))?;
    categories.map_err(|e| format!("categories: {e}"))
}

/// Prints a loaded list; its failure, if any, is the error.
fn print_list<T>(state: &ListState<T>, print: fn(&T)) -> Result<()> {
    match state {
        ListState::Loaded { items, more, .. } => {
            if items.is_empty() {
                println!("  (none)");
            }
            items.iter().for_each(print);
            match more {
                MorePages::Failed { message } => Err(format!("loading the next page: {message}")),
                MorePages::Idle | MorePages::Loading => Ok(()),
            }
        }
        ListState::FirstPage(FirstPage::Failed { message }) => Err(message.clone()),
        ListState::Idle | ListState::FirstPage(FirstPage::Loading) => {
            Err("nothing was loaded".to_owned())
        }
    }
}

fn print_channel(found: &FoundChannel) {
    let live = found
        .viewers
        .map_or_else(String::new, |viewers| format!("live, {viewers} viewers"));
    let followers = found
        .followers
        .map_or_else(String::new, |count| count.to_string());
    println!(
        "  {:<26} {:<26} {:>10}  {live}",
        found.channel.login.as_str(),
        found.channel.display_name,
        followers
    );
}

fn print_category(category: &Category) {
    println!(
        "  {:>10}  {:<32} {:>8}",
        category.id.as_str(),
        category.name,
        category.viewers
    );
}

async fn fetch_channels(twitch: &Twitch, search: &mut SearchController, request: PageRequest) {
    let Some(query) = search.query().cloned() else {
        return;
    };
    let result = twitch
        .search_channels(&query, request.cursor.as_ref())
        .await
        .map_err(|e| e.to_string());
    search.apply_channels(request, result);
}

async fn fetch_categories(twitch: &Twitch, search: &mut SearchController, request: PageRequest) {
    let Some(query) = search.query().cloned() else {
        return;
    };
    let result = twitch
        .search_categories(&query, request.cursor.as_ref())
        .await
        .map_err(|e| e.to_string());
    search.apply_categories(request, result);
}
