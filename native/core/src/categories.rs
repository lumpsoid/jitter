//! The categories directory: the top categories, paged by the shared
//! [`PagedList`], and the categories the live favorites are streaming.

use std::cmp::Reverse;

use crate::favorites::{FavoriteEntry, Liveness};
use crate::models::{Category, FavoritesCategory, Stream};
use crate::paged_list::PagedList;

pub type TopCategoriesController = PagedList<Category>;

/// The categories the live favorites in `entries` are streaming, each with
/// those streams. Favorites that are offline, unchecked or in no category
/// are left out, so no favorites live means no categories.
///
/// Categories go by their favorites' summed viewers, most first; ties by
/// name (ignoring case), then id. Within one, streams go by viewers, most
/// first; ties by login. Ids and logins are unique, so both orders are
/// total.
pub fn favorites_by_category(entries: &[FavoriteEntry]) -> Vec<FavoritesCategory<'_>> {
    let mut groups: Vec<FavoritesCategory<'_>> = Vec::new();
    for stream in entries.iter().filter_map(live_stream) {
        let Some(game) = &stream.game else {
            continue;
        };
        match groups.iter_mut().find(|g| g.game.id == game.id) {
            Some(group) => group.streams.push(stream),
            None => groups.push(FavoritesCategory {
                game,
                streams: vec![stream],
            }),
        }
    }
    for group in &mut groups {
        sort_by_viewers(&mut group.streams);
    }
    groups.sort_by_cached_key(|g| {
        (
            Reverse(g.viewers()),
            g.game.name.to_lowercase(),
            g.game.id.as_str().to_owned(),
        )
    });
    groups
}

pub(crate) fn live_stream(entry: &FavoriteEntry) -> Option<&Stream> {
    match &entry.liveness {
        Liveness::Live(stream) => Some(stream),
        Liveness::Unknown | Liveness::Offline => None,
    }
}

/// Most viewers first; ties by login, which is unique.
pub(crate) fn sort_by_viewers(streams: &mut [&Stream]) {
    streams.sort_by(|a, b| {
        b.viewers
            .cmp(&a.viewers)
            .then_with(|| a.channel.login.as_str().cmp(b.channel.login.as_str()))
    });
}
