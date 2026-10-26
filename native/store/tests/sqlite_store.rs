//! `SqliteStore` against the port contract core's fakes define, the
//! controllers over it, and two connections on one file standing in for two
//! heads (the GUI and a CLI) sharing the database.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};
use std::thread;

use jitter_core::favorites::FavoritesController;
use jitter_core::models::{
    Channel, ChannelId, ChannelLogin, Favorite, ResumePoint, Seconds, Stream, Timestamp, VideoId,
};
use jitter_core::ports::{FavoritesStore, ResumeStore};
use jitter_core::resume::ResumeController;
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
        added_at: Timestamp::from_unix(added_at),
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
        "INSERT INTO favorites VALUES ('not-a-number', 'a', 'A', 0)",
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
    assert_eq!(user_version(&path)?, 1);
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
        assert_eq!(user_version(&path)?, 1);
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
