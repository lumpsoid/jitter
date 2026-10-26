use jitter_core::fakes::FakeFavoritesStore;
use jitter_core::favorites::{FavoritesController, Liveness};
use jitter_core::models::{Channel, ChannelId, ChannelLogin, Favorite, Stream, Timestamp};

fn id(raw: &str) -> ChannelId {
    let Ok(id) = ChannelId::parse(raw) else {
        unreachable!("valid id");
    };
    id
}

fn login(raw: &str) -> ChannelLogin {
    let Ok(login) = ChannelLogin::parse(raw) else {
        unreachable!("valid login");
    };
    login
}

fn channel(raw_id: &str, name: &str) -> Channel {
    Channel {
        id: id(raw_id),
        login: login(&name.to_lowercase()),
        display_name: name.to_owned(),
        avatar_url: String::new(),
    }
}

fn favorite(raw_id: &str, name: &str) -> Favorite {
    let c = channel(raw_id, name);
    Favorite {
        channel_id: c.id,
        login: c.login,
        display_name: c.display_name,
        added_at: Timestamp::from_unix(0),
    }
}

fn stream(channel: Channel, viewers: u32) -> Stream {
    Stream {
        id: format!("s{}", channel.id),
        channel,
        title: String::new(),
        game_name: None,
        viewers,
        started_at: None,
        thumbnail_url: String::new(),
    }
}

fn loaded(store: &FakeFavoritesStore) -> FavoritesController {
    let mut controller = FavoritesController::new(Box::new(store.clone()));
    assert_eq!(controller.load(), Ok(()));
    controller
}

fn names(controller: &FavoritesController) -> Vec<String> {
    controller
        .entries()
        .iter()
        .map(|e| e.favorite.display_name.clone())
        .collect()
}

#[test]
fn adding_the_same_channel_twice_keeps_one_entry() {
    let store = FakeFavoritesStore::default();
    let mut controller = loaded(&store);
    let now = Timestamp::from_unix(100);
    assert_eq!(controller.add(channel("1", "Alpha"), now), Ok(()));
    assert_eq!(controller.add(channel("1", "Alpha"), now), Ok(()));
    assert_eq!(controller.entries().len(), 1);
    assert_eq!(store.favorites().len(), 1);
    assert_eq!(store.favorites()[0].added_at, now);
}

#[test]
fn a_failing_store_leaves_the_state_unchanged_and_returns_err() {
    let store = FakeFavoritesStore::with_favorites(vec![favorite("1", "Alpha")]).failing_writes();
    let mut controller = loaded(&store);
    let before = controller.entries().to_vec();

    assert!(
        controller
            .add(channel("2", "Beta"), Timestamp::from_unix(1))
            .is_err()
    );
    assert_eq!(controller.entries(), before.as_slice());

    assert!(controller.remove(&id("1")).is_err());
    assert_eq!(controller.entries(), before.as_slice());
    assert_eq!(store.favorites().len(), 1);
}

#[test]
fn a_failing_load_returns_err() {
    let store = FakeFavoritesStore::with_favorites(vec![favorite("1", "Alpha")]).failing_loads();
    let mut controller = FavoritesController::new(Box::new(store));
    assert!(controller.load().is_err());
    assert!(controller.entries().is_empty());
}

#[test]
fn apply_live_sorts_live_by_viewers_then_offline_by_name() {
    let store = FakeFavoritesStore::with_favorites(vec![
        favorite("1", "delta"),
        favorite("2", "Bravo"),
        favorite("3", "charlie"),
        favorite("4", "alpha"),
    ]);
    let mut controller = loaded(&store);
    let result = controller.apply_live(vec![
        stream(channel("3", "charlie"), 10),
        stream(channel("1", "delta"), 500),
    ]);
    assert_eq!(result, Ok(()));
    assert_eq!(names(&controller), ["delta", "charlie", "alpha", "Bravo"]);
    let liveness: Vec<bool> = controller
        .entries()
        .iter()
        .map(|e| matches!(e.liveness, Liveness::Live(_)))
        .collect();
    assert_eq!(liveness, [true, true, false, false]);
    assert_eq!(controller.entries()[2].liveness, Liveness::Offline);
}

#[test]
fn apply_live_with_a_stream_for_a_non_favorite_ignores_it() {
    let store = FakeFavoritesStore::with_favorites(vec![favorite("1", "alpha")]);
    let mut controller = loaded(&store);
    assert_eq!(
        controller.apply_live(vec![stream(channel("9", "stranger"), 99)]),
        Ok(())
    );
    assert_eq!(names(&controller), ["alpha"]);
    assert_eq!(controller.entries()[0].liveness, Liveness::Offline);
    assert!(store.renamed().is_empty());
    assert_eq!(store.favorites().len(), 1);
}

#[test]
fn a_renamed_channels_new_login_is_written_through() {
    let store = FakeFavoritesStore::with_favorites(vec![favorite("1", "oldname")]);
    let mut controller = loaded(&store);
    assert_eq!(
        controller.apply_live(vec![stream(channel("1", "NewName"), 5)]),
        Ok(())
    );
    let entry = &controller.entries()[0];
    assert_eq!(entry.favorite.login, login("newname"));
    assert_eq!(entry.favorite.display_name, "NewName");
    let stored = store.favorites();
    assert_eq!(stored[0].login, login("newname"));
    assert_eq!(stored[0].display_name, "NewName");
}

#[test]
fn a_failing_rename_write_still_applies_live_status_and_returns_err() {
    let store = FakeFavoritesStore::with_favorites(vec![favorite("1", "oldname")]).failing_writes();
    let mut controller = loaded(&store);
    let result = controller.apply_live(vec![stream(channel("1", "NewName"), 5)]);
    assert!(result.is_err());
    let entry = &controller.entries()[0];
    assert!(matches!(entry.liveness, Liveness::Live(_)));
    assert_eq!(entry.favorite.login, login("oldname"));
    assert_eq!(entry.favorite.display_name, "oldname");
    assert_eq!(store.favorites()[0].login, login("oldname"));
}

#[test]
fn find_by_login_finds_by_the_cached_login() {
    let store =
        FakeFavoritesStore::with_favorites(vec![favorite("1", "alpha"), favorite("2", "beta")]);
    let controller = loaded(&store);
    assert_eq!(controller.find_by_login(&login("beta")), Some(id("2")));
    assert_eq!(controller.find_by_login(&login("gamma")), None);
}

#[test]
fn remove_then_apply_live_with_its_stream_does_not_bring_it_back() {
    let store =
        FakeFavoritesStore::with_favorites(vec![favorite("1", "alpha"), favorite("2", "beta")]);
    let mut controller = loaded(&store);
    let in_flight = controller.ids();
    assert_eq!(in_flight.len(), 2);
    assert_eq!(controller.remove(&id("1")), Ok(()));
    // The fetch started before the removal lands now, renamed to force a write.
    assert_eq!(
        controller.apply_live(vec![stream(channel("1", "Alpha2"), 50)]),
        Ok(())
    );
    assert_eq!(controller.ids(), [id("2")]);
    assert!(store.renamed().is_empty());
    assert_eq!(store.favorites().len(), 1);
}

#[test]
fn rename_is_called_only_for_a_channel_whose_login_or_name_changed() {
    let store = FakeFavoritesStore::with_favorites(vec![
        favorite("1", "same"),
        favorite("2", "oldlogin"),
        favorite("3", "case"),
    ]);
    let mut controller = loaded(&store);
    let mut recased = channel("3", "case");
    recased.display_name = "CASE".to_owned();
    assert_eq!(
        controller.apply_live(vec![
            stream(channel("1", "same"), 1),
            stream(channel("2", "newlogin"), 2),
            stream(recased, 3),
        ]),
        Ok(())
    );
    let mut renamed = store.renamed();
    renamed.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    assert_eq!(renamed, [id("2"), id("3")]);
}

#[test]
fn the_fake_store_never_resurrects_a_removed_favorite_on_rename() {
    use jitter_core::ports::FavoritesStore;
    let mut store = FakeFavoritesStore::with_favorites(vec![favorite("1", "alpha")]);
    assert_eq!(store.remove(&id("1")), Ok(()));
    assert_eq!(store.rename(&id("1"), &login("beta"), "beta"), Ok(()));
    assert!(store.favorites().is_empty());
}

#[test]
fn load_again_keeps_live_status_of_channels_still_stored() {
    let store = FakeFavoritesStore::with_favorites(vec![favorite("1", "alpha")]);
    let mut controller = loaded(&store);
    assert_eq!(
        controller.apply_live(vec![stream(channel("1", "alpha"), 7)]),
        Ok(())
    );
    assert_eq!(controller.load(), Ok(()));
    assert!(matches!(
        controller.entries()[0].liveness,
        Liveness::Live(_)
    ));
}
