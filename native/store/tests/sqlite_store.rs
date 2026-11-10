//! `SqliteStore` against the port contract core's fakes define, the
//! controllers over it, and two connections on one file standing in for two
//! heads (the GUI and a CLI) sharing the database.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};
use std::thread;

use jitter_core::favorites::FavoritesController;
use jitter_core::models::{
    CategoryId, Channel, ChannelId, ChannelLogin, Favorite, GameRef, NewVisit, QualityPreference,
    ResumePoint, Seconds, Setting, Settings, Stream, ThemeChoice, Timestamp, VideoId, VisitDetails,
    VisitId,
};
use jitter_core::playback::PlaybackTarget;
use jitter_core::ports::{FavoritesStore, HistoryStore, ResumeStore, SettingsStore};
use jitter_core::resume::ResumeController;
use jitter_core::settings::SettingsController;
use jitter_store::SqliteStore;
use rusqlite::Connection;
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

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

fn video(raw: &str) -> VideoId {
    let Ok(id) = VideoId::parse(raw) else {
        unreachable!("valid video id");
    };
    id
}

fn channel(raw_id: &str, name: &str) -> Channel {
    Channel {
        id: id(raw_id),
        login: login(&name.to_lowercase()),
        display_name: name.to_owned(),
        avatar_url: String::new(),
    }
}

fn favorite(raw_id: &str, name: &str, added_at: i64) -> Favorite {
    let c = channel(raw_id, name);
    Favorite {
        channel_id: c.id,
        login: c.login,
        display_name: c.display_name,
        avatar_url: None,
        added_at: Timestamp::from_unix(added_at),
    }
}

fn with_avatar(mut channel: Channel, url: &str) -> Channel {
    url.clone_into(&mut channel.avatar_url);
    channel
}

fn stream(channel: Channel, viewers: u32) -> Stream {
    Stream {
        id: format!("s{}", channel.id),
        channel,
        title: String::new(),
        game: None,
        viewers,
        started_at: None,
        thumbnail_url: String::new(),
    }
}

fn point(raw_id: &str, position: u32, duration: u32, saved_at: i64) -> ResumePoint {
    ResumePoint {
        video_id: video(raw_id),
        position: Seconds::new(position),
        duration: Seconds::new(duration),
        saved_at: Timestamp::from_unix(saved_at),
    }
}

/// A temp directory and the path of a database file in it that does not
/// exist yet. The directory is removed when the `TempDir` drops.
fn fresh_file() -> Result<(TempDir, PathBuf), std::io::Error> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("jitter.db");
    Ok((dir, path))
}

fn user_version(path: &Path) -> rusqlite::Result<u32> {
    Connection::open(path)?.pragma_query_value(None, "user_version", |r| r.get(0))
}

// --- Favorites -----------------------------------------------------------

#[test]
fn favorites_round_trip() -> TestResult {
    let mut store = SqliteStore::open_in_memory()?;
    let a = favorite("1", "Alpha", 10);
    let b = favorite("2", "beta_2", 20);
    store.add(&a)?;
    store.add(&b)?;
    assert_eq!(store.load_all()?, [a, b]);
    Ok(())
}

#[test]
fn adding_the_same_id_twice_keeps_the_first_row() -> TestResult {
    let mut store = SqliteStore::open_in_memory()?;
    let first = favorite("1", "Alpha", 10);
    store.add(&first)?;
    store.add(&favorite("1", "Other", 99))?;
    assert_eq!(store.load_all()?, [first]);
    Ok(())
}

#[test]
fn rename_updates_login_and_display_name_only() -> TestResult {
    let mut store = SqliteStore::open_in_memory()?;
    store.add(&favorite("1", "Alpha", 10))?;
    store.rename(&id("1"), &login("gamma"), "Gamma")?;
    assert_eq!(store.load_all()?, [favorite("1", "Gamma", 10)]);
    Ok(())
}

#[test]
fn rename_of_an_absent_id_is_ok_and_inserts_nothing() -> TestResult {
    let mut store = SqliteStore::open_in_memory()?;
    store.rename(&id("1"), &login("alpha"), "Alpha")?;
    assert!(store.load_all()?.is_empty());
    Ok(())
}

#[test]
fn remove_deletes_and_an_unknown_id_is_ok() -> TestResult {
    let mut store = SqliteStore::open_in_memory()?;
    store.add(&favorite("1", "Alpha", 10))?;
    store.remove(&id("1"))?;
    store.remove(&id("2"))?;
    assert!(store.load_all()?.is_empty());
    Ok(())
}

#[test]
fn a_corrupt_row_fails_the_load() -> TestResult {
    let (_dir, path) = fresh_file()?;
    let store = SqliteStore::open(&path)?;
    Connection::open(&path)?.execute(
        "INSERT INTO favorites VALUES ('not-a-number', 'a', 'A', 0, NULL)",
        [],
    )?;
    assert!(store.load_all().is_err());
    Ok(())
}

#[test]
fn favorites_controller_over_the_store() -> TestResult {
    let store = SqliteStore::open_in_memory()?;
    let mut controller = FavoritesController::new(Box::new(store.clone()));
    controller.load()?;
    controller.add(channel("1", "Alpha"), Timestamp::from_unix(5))?;
    controller.apply_live(vec![stream(channel("1", "AlphaNew"), 7)])?;
    assert_eq!(store.load_all()?, [favorite("1", "AlphaNew", 5)]);
    Ok(())
}

#[test]
fn set_avatar_updates_the_avatar_only_and_never_inserts() -> TestResult {
    let mut store = SqliteStore::open_in_memory()?;
    store.add(&favorite("1", "Alpha", 10))?;
    store.set_avatar(&id("1"), "https://a/1.png")?;
    store.set_avatar(&id("2"), "https://a/2.png")?;
    let expected = Favorite {
        avatar_url: Some("https://a/1.png".to_owned()),
        ..favorite("1", "Alpha", 10)
    };
    assert_eq!(store.load_all()?, [expected]);
    Ok(())
}

#[test]
fn the_controller_saves_the_avatar_on_add_and_refreshes_it_when_live() -> TestResult {
    let store = SqliteStore::open_in_memory()?;
    let mut controller = FavoritesController::new(Box::new(store.clone()));
    controller.load()?;
    controller.add(
        with_avatar(channel("1", "Alpha"), "https://a/old.png"),
        Timestamp::from_unix(5),
    )?;
    let avatar = |store: &SqliteStore| -> Result<Option<String>, Box<dyn std::error::Error>> {
        Ok(store.load_all()?.first().and_then(|f| f.avatar_url.clone()))
    };
    assert_eq!(avatar(&store)?.as_deref(), Some("https://a/old.png"));
    controller.apply_live(vec![stream(
        with_avatar(channel("1", "Alpha"), "https://a/new.png"),
        7,
    )])?;
    assert_eq!(avatar(&store)?.as_deref(), Some("https://a/new.png"));
    Ok(())
}

// --- Resume points -------------------------------------------------------

#[test]
fn resume_points_round_trip() -> TestResult {
    let mut store = SqliteStore::open_in_memory()?;
    let p = point("100", 1800, 3600, 42);
    store.put(&p)?;
    assert_eq!(store.get(&video("100"))?, Some(p));
    assert_eq!(store.get(&video("101"))?, None);
    Ok(())
}

#[test]
fn put_replaces_the_point_for_the_same_video() -> TestResult {
    let mut store = SqliteStore::open_in_memory()?;
    store.put(&point("100", 1800, 3600, 42))?;
    store.put(&point("100", 60, 3600, 43))?;
    assert_eq!(store.get(&video("100"))?, Some(point("100", 60, 3600, 43)));
    Ok(())
}

#[test]
fn clear_deletes_and_an_unknown_id_is_ok() -> TestResult {
    let mut store = SqliteStore::open_in_memory()?;
    store.put(&point("100", 1800, 3600, 42))?;
    store.clear(&video("100"))?;
    store.clear(&video("101"))?;
    assert_eq!(store.get(&video("100"))?, None);
    Ok(())
}

#[test]
fn resume_controller_over_the_store() -> TestResult {
    let store = SqliteStore::open_in_memory()?;
    let mut controller = ResumeController::new(Box::new(store.clone()));
    controller.record(
        video("100"),
        Seconds::new(1800),
        Seconds::new(3600),
        Timestamp::from_unix(1),
    )?;
    assert_eq!(
        controller.start_position(&video("100"))?,
        Seconds::new(1800)
    );
    Ok(())
}

// --- Migrations on a file ------------------------------------------------

#[test]
fn reopening_a_migrated_file_keeps_its_rows() -> TestResult {
    let (_dir, path) = fresh_file()?;
    let mut store = SqliteStore::open(&path)?;
    store.add(&favorite("1", "Alpha", 10))?;
    drop(store);
    let store = SqliteStore::open(&path)?;
    assert_eq!(store.load_all()?, [favorite("1", "Alpha", 10)]);
    assert_eq!(user_version(&path)?, 5);
    Ok(())
}

#[test]
fn a_v1_file_is_migrated_and_its_favorites_load_without_an_avatar() -> TestResult {
    let (_dir, path) = fresh_file()?;
    let v1 = Connection::open(&path)?;
    v1.execute_batch(
        "CREATE TABLE favorites (
             channel_id   TEXT PRIMARY KEY NOT NULL,
             login        TEXT NOT NULL,
             display_name TEXT NOT NULL,
             added_at     INTEGER NOT NULL
         );
         CREATE TABLE resume_points (
             video_id   TEXT PRIMARY KEY NOT NULL,
             position_s INTEGER NOT NULL,
             duration_s INTEGER NOT NULL,
             saved_at   INTEGER NOT NULL
         );
         INSERT INTO favorites VALUES ('1', 'alpha', 'Alpha', 10);
         PRAGMA user_version = 1;",
    )?;
    drop(v1);
    let mut store = SqliteStore::open(&path)?;
    assert_eq!(user_version(&path)?, 5);
    assert_eq!(store.load_all()?, [favorite("1", "Alpha", 10)]);
    store.set_avatar(&id("1"), "https://a/1.png")?;
    let expected = Favorite {
        avatar_url: Some("https://a/1.png".to_owned()),
        ..favorite("1", "Alpha", 10)
    };
    assert_eq!(store.load_all()?, [expected]);
    Ok(())
}

#[test]
fn a_file_with_a_newer_schema_is_refused() -> TestResult {
    let (_dir, path) = fresh_file()?;
    Connection::open(&path)?.pragma_update(None, "user_version", 99)?;
    assert!(SqliteStore::open(&path).is_err());
    assert_eq!(user_version(&path)?, 99);
    Ok(())
}

// --- History -------------------------------------------------------------

fn new_visit(target: PlaybackTarget, started_at: i64) -> NewVisit {
    NewVisit {
        target,
        started_at: Timestamp::from_unix(started_at),
        last_seen_at: Timestamp::from_unix(started_at + 60),
        details: VisitDetails::default(),
    }
}

fn live(raw: &str) -> PlaybackTarget {
    PlaybackTarget::Live(login(raw))
}

fn vod(raw: &str) -> PlaybackTarget {
    PlaybackTarget::Vod(video(raw))
}

#[test]
fn a_visit_round_trips_with_what_was_on() -> TestResult {
    let mut store = SqliteStore::open_in_memory()?;
    let Ok(game) = CategoryId::parse("509658") else {
        unreachable!("valid id");
    };
    let mut visit = new_visit(vod("42"), 1_000);
    visit.details = VisitDetails {
        channel: Some(with_avatar(
            channel("1", "Xqc"),
            "https://example.test/a.png",
        )),
        title: Some("react andy".to_owned()),
        game: Some(GameRef {
            id: game,
            name: "Just Chatting".to_owned(),
        }),
        streamed_at: Some(Timestamp::from_unix(500)),
    };
    let id = store.add_visit(&visit)?;
    let loaded = store.load_visits()?;
    let [stored] = loaded.as_slice() else {
        return Err(format!("expected one visit, got {loaded:?}").into());
    };
    assert_eq!(stored.id, id);
    assert_eq!(stored.target, visit.target);
    assert_eq!(stored.started_at, visit.started_at);
    assert_eq!(stored.last_seen_at, visit.last_seen_at);
    assert_eq!(stored.details, visit.details);
    assert_eq!(store.latest_visit()?.as_ref(), Some(stored));
    Ok(())
}

#[test]
fn a_visit_without_details_round_trips_empty() -> TestResult {
    let mut store = SqliteStore::open_in_memory()?;
    store.add_visit(&new_visit(live("xqc"), 1_000))?;
    let loaded = store.load_visits()?;
    assert_eq!(
        loaded.first().map(|v| &v.details),
        Some(&VisitDetails::default())
    );
    Ok(())
}

#[test]
fn visits_load_newest_start_first() -> TestResult {
    let mut store = SqliteStore::open_in_memory()?;
    store.add_visit(&new_visit(live("a"), 200))?;
    store.add_visit(&new_visit(live("b"), 100))?;
    store.add_visit(&new_visit(live("c"), 300))?;
    let starts: Vec<i64> = store
        .load_visits()?
        .iter()
        .map(|v| v.started_at.unix())
        .collect();
    assert_eq!(starts, [300, 200, 100]);
    assert_eq!(store.latest_visit()?.map(|v| v.target), Some(live("c")));
    Ok(())
}

#[test]
fn touch_moves_last_seen_forward_only_and_never_inserts() -> TestResult {
    let mut store = SqliteStore::open_in_memory()?;
    let id = store.add_visit(&new_visit(live("a"), 1_000))?;
    assert!(store.touch_visit(id, Timestamp::from_unix(2_000))?);
    assert!(store.touch_visit(id, Timestamp::from_unix(1_500))?);
    assert_eq!(
        store.latest_visit()?.map(|v| v.last_seen_at),
        Some(Timestamp::from_unix(2_000))
    );
    store.remove_visit(id)?;
    assert!(!store.touch_visit(id, Timestamp::from_unix(3_000))?);
    assert!(store.load_visits()?.is_empty());
    Ok(())
}

#[test]
fn a_removed_visit_id_is_never_given_out_again() -> TestResult {
    let mut store = SqliteStore::open_in_memory()?;
    let first = store.add_visit(&new_visit(live("a"), 1_000))?;
    store.remove_visit(first)?;
    let second = store.add_visit(&new_visit(live("b"), 2_000))?;
    assert_ne!(first, second);
    store.clear_visits()?;
    let third = store.add_visit(&new_visit(live("c"), 3_000))?;
    assert!(third.get() > second.get());
    Ok(())
}

#[test]
fn remove_target_removes_that_targets_visits_only() -> TestResult {
    let mut store = SqliteStore::open_in_memory()?;
    store.add_visit(&new_visit(live("a"), 100))?;
    store.add_visit(&new_visit(vod("42"), 200))?;
    store.add_visit(&new_visit(live("a"), 300))?;
    store.remove_visits_to(&live("a"))?;
    let left: Vec<PlaybackTarget> = store.load_visits()?.into_iter().map(|v| v.target).collect();
    assert_eq!(left, [vod("42")]);
    store.remove_visit(VisitId::from_raw(999))?;
    store.clear_visits()?;
    assert!(store.load_visits()?.is_empty());
    Ok(())
}

#[test]
fn a_corrupt_visit_row_fails_the_load() -> TestResult {
    let (_dir, path) = fresh_file()?;
    let store = SqliteStore::open(&path)?;
    Connection::open(&path)?.execute(
        "INSERT INTO visits (kind, target, started_at, last_seen_at)
         VALUES ('vod', 'not a video id', 0, 0)",
        [],
    )?;
    assert!(store.load_visits().is_err());
    Ok(())
}

// --- Two connections on one file (two heads) -----------------------------

#[test]
fn the_file_is_in_wal_mode_after_open() -> TestResult {
    let (_dir, path) = fresh_file()?;
    let _store = SqliteStore::open(&path)?;
    let mode: String =
        Connection::open(&path)?.pragma_query_value(None, "journal_mode", |r| r.get(0))?;
    assert_eq!(mode, "wal");
    Ok(())
}

/// Two heads opening a fresh file at the same moment both succeed. With a
/// `DEFERRED` migration one of them fails with `SQLITE_BUSY` (both read
/// `user_version`, neither can upgrade its lock). Repeated because the race
/// is not hit on every run.
#[test]
fn two_connections_opening_a_fresh_file_at_once_both_succeed() -> TestResult {
    for _ in 0..25 {
        let (_dir, path) = fresh_file()?;
        let barrier = Arc::new(Barrier::new(2));
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let barrier = Arc::clone(&barrier);
                let path = path.clone();
                thread::spawn(move || {
                    barrier.wait();
                    SqliteStore::open(&path).map(drop)
                })
            })
            .collect();
        for handle in handles {
            let opened = handle.join().map_err(|_| "open thread panicked")?;
            assert_eq!(opened, Ok(()));
        }
        assert_eq!(user_version(&path)?, 5);
    }
    Ok(())
}

#[test]
fn a_favorite_added_through_one_connection_is_seen_by_the_other_after_load() -> TestResult {
    let (_dir, path) = fresh_file()?;
    let mut gui = FavoritesController::new(Box::new(SqliteStore::open(&path)?));
    let mut cli = FavoritesController::new(Box::new(SqliteStore::open(&path)?));
    gui.load()?;
    cli.add(channel("1", "Alpha"), Timestamp::from_unix(5))?;
    assert!(gui.entries().is_empty());
    gui.load()?;
    assert_eq!(gui.ids(), [id("1")]);
    Ok(())
}

/// A favorite removed through A (a CLI `fav rm`) while B (the GUI) still
/// lists it: B's rename write-through for the renamed live channel must not
/// bring it back.
#[test]
fn a_rename_through_one_connection_does_not_resurrect_a_removal_through_the_other() -> TestResult {
    let (_dir, path) = fresh_file()?;
    let a_store = SqliteStore::open(&path)?;
    let mut a = FavoritesController::new(Box::new(a_store.clone()));
    let mut b = FavoritesController::new(Box::new(SqliteStore::open(&path)?));
    a.add(channel("1", "Alpha"), Timestamp::from_unix(5))?;
    b.load()?;
    a.remove(&id("1"))?;
    b.apply_live(vec![stream(channel("1", "AlphaNew"), 7)])?;
    assert!(a_store.load_all()?.is_empty());
    Ok(())
}

#[test]
fn settings_never_stored_are_the_defaults() -> TestResult {
    let store = SqliteStore::open_in_memory()?;
    assert_eq!(store.load_settings()?, Settings::default());
    Ok(())
}

#[test]
fn every_setting_round_trips() -> TestResult {
    let mut store = SqliteStore::open_in_memory()?;
    for setting in [
        Setting::Theme(ThemeChoice::Light),
        Setting::AnimatedEmotes(false),
        Setting::AutoLoadChat(false),
        Setting::StartQuality(QualityPreference::AudioOnly),
        Setting::ThirdPartyEmotes(false),
        Setting::BackgroundAudio(false),
    ] {
        store.put_setting(setting)?;
    }
    let expected = Settings {
        theme: ThemeChoice::Light,
        animated_emotes: false,
        auto_load_chat: false,
        start_quality: QualityPreference::AudioOnly,
        third_party_emotes: false,
        background_audio: false,
    };
    assert_eq!(store.load_settings()?, expected);
    store.put_setting(Setting::AnimatedEmotes(true))?;
    assert_eq!(
        store.load_settings()?,
        Settings {
            animated_emotes: true,
            ..expected
        }
    );
    Ok(())
}

#[test]
fn a_corrupt_setting_fails_the_load() -> TestResult {
    let (_dir, path) = fresh_file()?;
    let store = SqliteStore::open(&path)?;
    Connection::open(&path)?.execute("UPDATE settings SET start_quality = '1440p'", [])?;
    assert!(store.load_settings().is_err());
    Connection::open(&path)?.execute(
        "UPDATE settings SET start_quality = NULL, auto_load_chat = 2",
        [],
    )?;
    assert!(store.load_settings().is_err());
    Ok(())
}

#[test]
fn a_setting_changed_through_one_connection_survives_a_change_through_the_other() -> TestResult {
    let (_dir, path) = fresh_file()?;
    let mut gui = SettingsController::new(Box::new(SqliteStore::open(&path)?));
    let mut cli = SettingsController::new(Box::new(SqliteStore::open(&path)?));
    gui.load()?;
    cli.load()?;
    cli.set(Setting::Theme(ThemeChoice::Dark))?;
    gui.set(Setting::StartQuality(QualityPreference::P720))?;
    gui.load()?;
    assert_eq!(gui.state().theme, ThemeChoice::Dark);
    assert_eq!(gui.state().start_quality, QualityPreference::P720);
    Ok(())
}

#[test]
fn a_visit_one_connection_removed_stays_removed_when_the_other_touches_it() -> TestResult {
    let (_dir, path) = fresh_file()?;
    let mut gui = SqliteStore::open(&path)?;
    let mut cli = SqliteStore::open(&path)?;
    let id = gui.add_visit(&new_visit(live("a"), 1_000))?;
    cli.clear_visits()?;
    assert!(!gui.touch_visit(id, Timestamp::from_unix(2_000))?);
    assert!(cli.load_visits()?.is_empty());
    Ok(())
}

// --- Backups -------------------------------------------------------------

/// The visits without the ids their database gave them.
fn visits_of(store: &SqliteStore) -> Result<Vec<NewVisit>, jitter_store::StoreError> {
    Ok(store
        .load_visits()?
        .into_iter()
        .map(|v| NewVisit {
            target: v.target,
            started_at: v.started_at,
            last_seen_at: v.last_seen_at,
            details: v.details,
        })
        .collect())
}

/// A store holding one of everything a backup carries.
fn kept_store() -> Result<SqliteStore, Box<dyn std::error::Error>> {
    let mut store = SqliteStore::open_in_memory()?;
    store.add(&Favorite {
        avatar_url: Some("https://a/1.png".to_owned()),
        ..favorite("1", "Alpha", 10)
    })?;
    let Ok(game) = CategoryId::parse("509658") else {
        unreachable!("valid id");
    };
    let mut watched = new_visit(vod("42"), 1_000);
    watched.details = VisitDetails {
        channel: Some(with_avatar(channel("1", "Alpha"), "https://a/1.png")),
        title: Some("react andy".to_owned()),
        game: Some(GameRef {
            id: game,
            name: "Just Chatting".to_owned(),
        }),
        streamed_at: Some(Timestamp::from_unix(500)),
    };
    store.add_visit(&watched)?;
    store.add_visit(&new_visit(live("alpha"), 2_000))?;
    store.put(&point("42", 300, 3_600, 1_100))?;
    store.put_setting(Setting::Theme(ThemeChoice::Dark))?;
    store.put_setting(Setting::StartQuality(QualityPreference::P720))?;
    store.put_setting(Setting::BackgroundAudio(false))?;
    Ok(store)
}

#[test]
fn a_backup_restores_everything_into_an_empty_database() -> TestResult {
    let kept = kept_store()?;
    let backup = kept.export_backup(Timestamp::from_unix(9_999))?;

    let mut restored = SqliteStore::open_in_memory()?;
    let imported = restored.import_backup(&backup)?;

    assert_eq!(
        imported,
        jitter_store::Imported {
            favorites: 1,
            visits: 2,
            resume_points: 1,
        }
    );
    assert_eq!(restored.load_all()?, kept.load_all()?);
    assert_eq!(visits_of(&restored)?, visits_of(&kept)?);
    assert_eq!(restored.get(&video("42"))?, kept.get(&video("42"))?);
    assert_eq!(restored.load_settings()?, kept.load_settings()?);
    Ok(())
}

#[test]
fn importing_a_backup_again_adds_nothing() -> TestResult {
    let mut store = kept_store()?;
    let backup = store.export_backup(Timestamp::from_unix(9_999))?;
    let visits = visits_of(&store)?;

    let imported = store.import_backup(&backup)?;

    assert_eq!(imported, jitter_store::Imported::default());
    assert_eq!(visits_of(&store)?, visits);
    assert_eq!(store.load_all()?.len(), 1);
    Ok(())
}

#[test]
fn an_import_keeps_what_is_stored_and_only_a_newer_resume_point_replaces() -> TestResult {
    let mut source = SqliteStore::open_in_memory()?;
    source.add(&favorite("1", "OldName", 1))?;
    source.add(&favorite("2", "Beta", 2))?;
    source.put(&point("42", 100, 3_600, 500))?;
    source.put(&point("43", 900, 3_600, 900))?;
    let backup = source.export_backup(Timestamp::from_unix(1_000))?;

    let mut store = SqliteStore::open_in_memory()?;
    store.add(&favorite("1", "Alpha", 10))?;
    store.put(&point("42", 200, 3_600, 600))?;
    store.put(&point("43", 50, 3_600, 100))?;
    let imported = store.import_backup(&backup)?;

    assert_eq!(imported.favorites, 1);
    assert_eq!(imported.resume_points, 1);
    assert_eq!(
        store.load_all()?,
        [favorite("2", "Beta", 2), favorite("1", "Alpha", 10)]
    );
    assert_eq!(store.get(&video("42"))?, Some(point("42", 200, 3_600, 600)));
    assert_eq!(store.get(&video("43"))?, Some(point("43", 900, 3_600, 900)));
    Ok(())
}

#[test]
fn a_backup_without_a_setting_leaves_that_setting_as_stored() -> TestResult {
    let mut store = SqliteStore::open_in_memory()?;
    store.put_setting(Setting::AnimatedEmotes(false))?;
    store.import_backup(
        r#"{"format": "jitter-backup", "version": 1, "exported_at": 0,
            "settings": {"theme": "light"}}"#,
    )?;
    assert_eq!(
        store.load_settings()?,
        Settings {
            theme: ThemeChoice::Light,
            animated_emotes: false,
            ..Settings::default()
        }
    );
    Ok(())
}

#[test]
fn a_backup_with_one_invalid_value_imports_nothing() -> TestResult {
    let mut store = SqliteStore::open_in_memory()?;
    let result = store.import_backup(
        r#"{"format": "jitter-backup", "version": 1, "exported_at": 0,
            "settings": {"theme": "dark"},
            "favorites": [
                {"channel_id": "1", "login": "alpha", "display_name": "Alpha",
                 "avatar_url": null, "added_at": 0},
                {"channel_id": "not-a-number", "login": "beta", "display_name": "Beta",
                 "avatar_url": null, "added_at": 0}
            ]}"#,
    );
    assert!(result.is_err());
    assert!(store.load_all()?.is_empty());
    assert_eq!(store.load_settings()?, Settings::default());
    Ok(())
}

#[test]
fn other_json_and_a_newer_backup_are_refused() -> TestResult {
    let mut store = SqliteStore::open_in_memory()?;
    assert!(store.import_backup(r#"{"favorites": []}"#).is_err());
    assert!(
        store
            .import_backup(r#"{"format": "other", "version": 1, "exported_at": 0}"#)
            .is_err()
    );
    let newer = store
        .import_backup(r#"{"format": "jitter-backup", "version": 2, "favorites": "changed"}"#)
        .err()
        .ok_or("a newer backup was imported")?;
    assert!(newer.reason().contains("version 2"), "{newer}");
    Ok(())
}
