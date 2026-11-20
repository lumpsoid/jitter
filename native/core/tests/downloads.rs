use jitter_core::download::{Download, DownloadProgress};
use jitter_core::downloads::{DownloadEntry, DownloadKey, DownloadStatus, DownloadsController};
use jitter_core::models::{SegmentList, VideoId};

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

fn start(controller: &mut DownloadsController) -> DownloadKey {
    let Some(key) = controller.start(vod(), &two_parts()) else {
        unreachable!("nothing else is running");
    };
    key
}

#[test]
fn start_lists_a_running_entry_with_its_file_and_progress() {
    let mut controller = DownloadsController::new();
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
fn the_same_file_cannot_start_twice_while_running() {
    let mut controller = DownloadsController::new();
    let first = start(&mut controller);

    assert!(controller.start(vod(), &two_parts()).is_none());

    assert!(controller.finish(first, "/out/a.ts".to_owned()));
    let Some(second) = controller.start(vod(), &two_parts()) else {
        unreachable!("the first download has ended");
    };
    assert_eq!(second.get(), 2);
}

#[test]
fn progress_and_finish_update_only_a_running_entry() {
    let mut controller = DownloadsController::new();
    let key = start(&mut controller);
    let advanced = DownloadProgress {
        done: 1,
        total: 2,
        bytes: 10,
    };

    assert!(controller.progress(key, advanced));
    assert!(!controller.progress(key, advanced));
    assert_eq!(controller.entries()[0].progress, advanced);

    assert!(controller.finish(key, "/out/a.ts".to_owned()));
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
}

#[test]
fn cancel_marks_running_cancelled_and_late_reports_are_ignored() {
    let mut controller = DownloadsController::new();
    let key = start(&mut controller);

    assert!(controller.cancel(key));
    assert_eq!(controller.entries()[0].status, DownloadStatus::Cancelled);

    assert!(!controller.finish(key, "/out/a.ts".to_owned()));
    assert_eq!(controller.entries()[0].status, DownloadStatus::Cancelled);
}

#[test]
fn remove_drops_only_ended_entries() {
    let mut controller = DownloadsController::new();
    let key = start(&mut controller);

    assert!(!controller.remove(key));
    assert_eq!(controller.entries().len(), 1);

    assert!(controller.fail(key, "network".to_owned()));
    assert!(controller.remove(key));
    assert!(controller.entries().is_empty());
}

#[test]
fn unknown_keys_change_nothing() {
    let mut controller = DownloadsController::new();
    let _ = start(&mut controller);
    let unknown = DownloadKey::new(99);
    let before = controller.entries().to_vec();

    let advanced = DownloadProgress {
        done: 1,
        total: 2,
        bytes: 10,
    };
    assert!(!controller.progress(unknown, advanced));
    assert!(!controller.finish(unknown, "/out/a.ts".to_owned()));
    assert!(!controller.fail(unknown, "network".to_owned()));
    assert!(!controller.cancel(unknown));
    assert!(!controller.remove(unknown));
    assert_eq!(controller.entries(), before);
}
