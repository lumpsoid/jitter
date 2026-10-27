use jitter_core::categories::favorites_by_category;
use jitter_core::category::{CategoryController, pin_favorites};
use jitter_core::favorites::{FavoriteEntry, Liveness};
use jitter_core::models::{
    CategoryId, Channel, ChannelId, ChannelLogin, Favorite, GameRef, ListState, Page, Stream,
    Timestamp,
};

fn category_id(raw: &str) -> CategoryId {
    CategoryId::parse(raw).unwrap_or_else(|e| unreachable!("{e:?}"))
}

fn game(id: &str, name: &str) -> GameRef {
    GameRef {
        id: category_id(id),
        name: name.to_owned(),
    }
}

fn channel(id: &str, login: &str) -> Channel {
    Channel {
        id: ChannelId::parse(id).unwrap_or_else(|e| unreachable!("{e:?}")),
        login: ChannelLogin::parse(login).unwrap_or_else(|e| unreachable!("{e:?}")),
        display_name: login.to_owned(),
        avatar_url: String::new(),
    }
}

fn stream(channel_id: &str, login: &str, game: Option<GameRef>, viewers: u32) -> Stream {
    Stream {
        id: format!("s{channel_id}"),
        channel: channel(channel_id, login),
        title: String::new(),
        game,
        viewers,
        started_at: None,
        thumbnail_url: String::new(),
    }
}

fn entry(channel_id: &str, login: &str, liveness: Liveness) -> FavoriteEntry {
    let c = channel(channel_id, login);
    FavoriteEntry {
        favorite: Favorite {
            channel_id: c.id,
            login: c.login,
            display_name: c.display_name,
            avatar_url: None,
            added_at: Timestamp::from_unix(0),
        },
        liveness,
    }
}

fn live(channel_id: &str, login: &str, game: Option<GameRef>, viewers: u32) -> FavoriteEntry {
    let s = stream(channel_id, login, game, viewers);
    entry(channel_id, login, Liveness::Live(Box::new(s)))
}

fn logins(streams: &[&Stream]) -> Vec<String> {
    streams
        .iter()
        .map(|s| s.channel.login.as_str().to_owned())
        .collect()
}

// --- From your channels ----------------------------------------------------

#[test]
fn no_live_favorites_means_no_categories() {
    let entries = [
        entry("1", "lirik", Liveness::Offline),
        entry("2", "vinesauce", Liveness::Unknown),
    ];
    assert!(favorites_by_category(&entries).is_empty());
}

#[test]
fn live_favorites_group_by_category() {
    let elden = game("512953", "Elden Ring");
    let entries = [
        live("1", "lirik", Some(elden.clone()), 10_000),
        live("2", "xqc", Some(game("509658", "Just Chatting")), 50_000),
        live("3", "vinesauce", Some(elden.clone()), 2_000),
        entry("4", "offline", Liveness::Offline),
    ];

    let groups = favorites_by_category(&entries);

    let summary: Vec<(&str, Vec<String>, u64)> = groups
        .iter()
        .map(|g| (g.game.name.as_str(), logins(&g.streams), g.viewers()))
        .collect();
    assert_eq!(
        summary,
        [
            ("Just Chatting", vec!["xqc".to_owned()], 50_000),
            (
                "Elden Ring",
                vec!["lirik".to_owned(), "vinesauce".to_owned()],
                12_000
            ),
        ]
    );
}

#[test]
fn categories_rank_by_summed_viewers_not_the_biggest_stream() {
    let entries = [
        live("1", "big", Some(game("1", "Solo")), 900),
        live("2", "a", Some(game("2", "Pair")), 500),
        live("3", "b", Some(game("2", "Pair")), 500),
    ];
    let names: Vec<&str> = favorites_by_category(&entries)
        .iter()
        .map(|g| g.game.name.as_str())
        .collect();
    assert_eq!(names, ["Pair", "Solo"]);
}

#[test]
fn equal_viewers_fall_back_to_name_then_id() {
    let entries = [
        live("1", "a", Some(game("30", "beta")), 100),
        live("2", "b", Some(game("20", "Alpha")), 100),
        live("3", "c", Some(game("10", "alpha")), 100),
    ];
    let ids: Vec<&str> = favorites_by_category(&entries)
        .iter()
        .map(|g| g.game.id.as_str())
        .collect();
    assert_eq!(ids, ["10", "20", "30"]);
}

#[test]
fn streams_in_a_category_rank_by_viewers_then_login() {
    let g = game("1", "Elden Ring");
    let entries = [
        live("1", "zed", Some(g.clone()), 100),
        live("2", "amy", Some(g.clone()), 100),
        live("3", "top", Some(g.clone()), 500),
    ];
    let groups = favorites_by_category(&entries);
    assert_eq!(groups.len(), 1);
    assert_eq!(logins(&groups[0].streams), ["top", "amy", "zed"]);
}

#[test]
fn a_live_favorite_without_a_category_is_left_out() {
    let entries = [live("1", "lirik", None, 10_000)];
    assert!(favorites_by_category(&entries).is_empty());
}

// --- Favorites pinned first ------------------------------------------------

#[test]
fn favorites_playing_the_category_are_pinned_first_and_not_repeated() {
    let elden = game("512953", "Elden Ring");
    let favorites = [
        live("1", "lirik", Some(elden.clone()), 3_000),
        live("2", "xqc", Some(game("509658", "Just Chatting")), 50_000),
        live("3", "vinesauce", Some(elden.clone()), 9_000),
        entry("4", "offline", Liveness::Offline),
    ];
    let page = [
        stream("10", "biggest", Some(elden.clone()), 80_000),
        stream("3", "vinesauce", Some(elden.clone()), 9_000),
        stream("11", "other", Some(elden.clone()), 1_000),
    ];

    let pinned = pin_favorites(&elden.id, &favorites, &page);

    assert_eq!(logins(&pinned.favorites), ["vinesauce", "lirik"]);
    assert_eq!(logins(&pinned.others), ["biggest", "other"]);
}

#[test]
fn without_favorites_in_the_category_the_page_is_unchanged() {
    let elden = game("512953", "Elden Ring");
    let favorites = [live("2", "xqc", Some(game("509658", "Just Chatting")), 1)];
    let page = [
        stream("10", "a", Some(elden.clone()), 2),
        stream("11", "b", Some(elden.clone()), 1),
    ];

    let pinned = pin_favorites(&elden.id, &favorites, &page);

    assert!(pinned.favorites.is_empty());
    assert_eq!(logins(&pinned.others), ["a", "b"]);
}

// --- The category's paged streams -------------------------------------------

#[test]
fn a_page_for_the_category_shown_before_is_ignored() {
    let mut category = CategoryController::new(category_id("1"));
    let old = category.begin_refresh();
    let new = category.show(category_id("2"));
    assert_eq!(category.id(), &category_id("2"));

    let page = |login: &str| Page {
        items: vec![stream("5", login, None, 1)],
        next: None,
    };
    category.apply_page(old, Ok(page("old")));
    assert!(!matches!(category.state(), ListState::Loaded { .. }));

    category.apply_page(new, Ok(page("new")));
    let ListState::Loaded { items, .. } = category.state() else {
        unreachable!("not loaded: {:?}", category.state());
    };
    assert_eq!(items[0].channel.login.as_str(), "new");
}
