use jitter_core::download::{Download, DownloadProgress};
use jitter_core::downloads::{DownloadEntry, DownloadKey, DownloadStatus, DownloadsController};
use jitter_core::fakes::FakeDownloadsStore;
use jitter_core::models::{FinishedDownload, SegmentList, VideoId};
use jitter_core::ports::StoreError;

type TestResult = Result<(), StoreError>;

fn vod() -> VideoId {
    let Ok(id) = VideoId::parse("2931745878") else {
        unreachable!("valid id");
    };
    id
}

fn two_parts() -> Download {
    let list = SegmentList {
        init: None,
        segments: vec!["s0".to_owned(), "s1".to_owned()],
    };
    Download::new(None, &vod(), "160p30", list)
}

fn controller() -> DownloadsController {
    DownloadsController::new(Box::new(FakeDownloadsStore::default()))
}

fn start(controller: &mut DownloadsController) -> DownloadKey {
    let Some(key) = controller.start(vod(), &two_parts()) else {
        unreachable!("nothing else is running");
    };
    key
}

#[test]
fn start_lists_a_running_entry_with_its_file_and_progress() {
    let mut controller = controller();
    let key = start(&mut controller);

    assert_eq!(key.get(), 1);
    assert_eq!(
        controller.entries(),
        [DownloadEntry {
            key,
            video: vod(),
            file_name: "2931745878_160p30.ts".to_owned(),
            progress: DownloadProgress {
                done: 0,
                total: 2,
                bytes: 0
            },
            status: DownloadStatus::Running,
        }]
    );
}

#[test]
fn the_same_file_cannot_start_twice_while_running() -> TestResult {
    let mut controller = controller();
    let first = start(&mut controller);

    assert!(controller.start(vod(), &two_parts()).is_none());

    assert!(controller.finish(first, "/out/a.ts".to_owned())?);
    let Some(second) = controller.start(vod(), &two_parts()) else {
        unreachable!("the first download has ended");
    };
    assert_eq!(second.get(), 2);
    Ok(())
}

#[test]
fn progress_and_finish_update_only_a_running_entry() -> TestResult {
    let mut controller = controller();
    let key = start(&mut controller);
    let advanced = DownloadProgress {
        done: 1,
        total: 2,
        bytes: 10,
    };

    assert!(controller.progress(key, advanced));
    assert!(!controller.progress(key, advanced));
    assert_eq!(controller.entries()[0].progress, advanced);

    assert!(controller.finish(key, "/out/a.ts".to_owned())?);
    assert_eq!(
        controller.entries()[0].status,
        DownloadStatus::Done {
            path: "/out/a.ts".to_owned()
        }
    );

    let later = DownloadProgress {
        done: 2,
        total: 2,
        bytes: 20,
    };
    assert!(!controller.progress(key, later));
    assert!(!controller.fail(key, "late".to_owned()));
    assert!(!controller.cancel(key));
    assert_eq!(
        controller.entries()[0].status,
        DownloadStatus::Done {
            path: "/out/a.ts".to_owned()
        }
    );
    assert_eq!(controller.entries()[0].progress, advanced);
    Ok(())
}

#[test]
fn cancel_marks_running_cancelled_and_late_reports_are_ignored() -> TestResult {
    let mut controller = controller();
    let key = start(&mut controller);

    assert!(controller.cancel(key));
    assert_eq!(controller.entries()[0].status, DownloadStatus::Cancelled);

    assert!(!controller.finish(key, "/out/a.ts".to_owned())?);
    assert_eq!(controller.entries()[0].status, DownloadStatus::Cancelled);
    Ok(())
}

#[test]
fn remove_drops_only_ended_entries() -> TestResult {
    let mut controller = controller();
    let key = start(&mut controller);

    assert!(!controller.remove(key)?);
    assert_eq!(controller.entries().len(), 1);

    assert!(controller.fail(key, "network".to_owned()));
    assert!(controller.remove(key)?);
    assert!(controller.entries().is_empty());
    Ok(())
}

#[test]
fn unknown_keys_change_nothing() -> TestResult {
    let mut controller = controller();
    let _ = start(&mut controller);
    let unknown = DownloadKey::new(99);
    let before = controller.entries().to_vec();

    let advanced = DownloadProgress {
        done: 1,
        total: 2,
        bytes: 10,
    };
    assert!(!controller.progress(unknown, advanced));
    assert!(!controller.finish(unknown, "/out/a.ts".to_owned())?);
    assert!(!controller.fail(unknown, "network".to_owned()));
    assert!(!controller.cancel(unknown));
    assert!(!controller.remove(unknown)?);
    assert_eq!(controller.entries(), before);
    Ok(())
}

fn finished(path: &str) -> FinishedDownload {
    FinishedDownload {
        video: vod(),
        file_name: "2931745878_160p30.ts".to_owned(),
        path: path.to_owned(),
        segments: 2,
        bytes: 20,
    }
}

#[test]
fn finish_stores_the_download_with_its_last_progress() -> TestResult {
    let store = FakeDownloadsStore::default();
    let mut controller = DownloadsController::new(Box::new(store.clone()));
    let key = start(&mut controller);
    let last = DownloadProgress {
        done: 2,
        total: 2,
        bytes: 20,
    };
    controller.progress(key, last);

    assert!(controller.finish(key, "/out/a.ts".to_owned())?);

    assert_eq!(store.downloads(), [finished("/out/a.ts")]);
    Ok(())
}

#[test]
fn a_finish_the_store_refuses_is_still_done() {
    let store = FakeDownloadsStore::default().failing_writes();
    let mut controller = DownloadsController::new(Box::new(store));
    let key = start(&mut controller);

    assert!(controller.finish(key, "/out/a.ts".to_owned()).is_err());

    assert_eq!(
        controller.entries()[0].status,
        DownloadStatus::Done {
            path: "/out/a.ts".to_owned()
        }
    );
}

#[test]
fn load_lists_the_stored_downloads_done_under_fresh_keys() -> TestResult {
    let store =
        FakeDownloadsStore::with_downloads(vec![finished("/out/a.ts"), finished("/out/b.ts")]);
    let mut controller = DownloadsController::new(Box::new(store));

    controller.load()?;
    let key = start(&mut controller);

    let paths: Vec<_> = controller
        .entries()
        .iter()
        .map(|e| (e.key.get(), e.status.clone()))
        .collect();
    assert_eq!(
        paths,
        [
            (
                1,
                DownloadStatus::Done {
                    path: "/out/a.ts".to_owned()
                }
            ),
            (
                2,
                DownloadStatus::Done {
                    path: "/out/b.ts".to_owned()
                }
            ),
            (3, DownloadStatus::Running),
        ]
    );
    assert_eq!(key.get(), 3);
    assert_eq!(
        controller.entries()[0].progress,
        DownloadProgress {
            done: 2,
            total: 2,
            bytes: 20
        }
    );
    Ok(())
}

#[test]
fn a_failed_load_leaves_the_list_as_it_was() {
    let store = FakeDownloadsStore::with_downloads(vec![finished("/out/a.ts")]).failing_loads();
    let mut controller = DownloadsController::new(Box::new(store));

    assert!(controller.load().is_err());
    assert!(controller.entries().is_empty());
}

#[test]
fn removing_a_done_entry_forgets_it_in_the_store() -> TestResult {
    let store =
        FakeDownloadsStore::with_downloads(vec![finished("/out/a.ts"), finished("/out/b.ts")]);
    let mut controller = DownloadsController::new(Box::new(store.clone()));
    controller.load()?;

    assert!(controller.remove(DownloadKey::new(1))?);

    assert_eq!(store.downloads(), [finished("/out/b.ts")]);
    assert_eq!(controller.entries().len(), 1);
    Ok(())
}

#[test]
fn a_remove_the_store_refuses_keeps_the_entry() -> TestResult {
    let store = FakeDownloadsStore::with_downloads(vec![finished("/out/a.ts")]);
    let mut controller = DownloadsController::new(Box::new(store.clone().failing_writes()));
    controller.load()?;

    assert!(controller.remove(DownloadKey::new(1)).is_err());

    assert_eq!(controller.entries().len(), 1);
    assert_eq!(store.downloads(), [finished("/out/a.ts")]);
    Ok(())
}

#[test]
fn downloaded_is_the_newest_finished_file_of_that_vod() -> TestResult {
    let other = FinishedDownload {
        video: VideoId::parse("1").map_err(|e| StoreError::new(format!("{e:?}")))?,
        ..finished("/out/other.ts")
    };
    let store = FakeDownloadsStore::with_downloads(vec![
        finished("/out/old.ts"),
        finished("/out/new.ts"),
        other,
    ]);
    let mut controller = DownloadsController::new(Box::new(store));
    controller.load()?;
    let _running = start(&mut controller);

    assert_eq!(controller.downloaded(&vod()), Some("/out/new.ts"));
    Ok(())
}

#[test]
fn a_vod_without_a_finished_download_has_no_file() {
    let mut controller = controller();
    let key = start(&mut controller);
    assert_eq!(controller.downloaded(&vod()), None);

    controller.fail(key, "network".to_owned());
    assert_eq!(controller.downloaded(&vod()), None);
}
