//! The downloads list: download signals ↔ the downloads controller, and one
//! writer task per running download, which fetches the VOD's segments and
//! joins them into one file.
//!
//! A download is resolved before it starts (playback, channel, segment list),
//! so the controller refuses a file that is already being written before any
//! byte is written. File writes run on tokio's blocking threads: the hub's
//! runtime is single-threaded and also carries every UI signal.
//!
//! The steps follow the CLI's `jitter download`, which this copies rather
//! than imports.

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use jitter_core::download::{Download, DownloadProgress};
use jitter_core::downloads::{DownloadEntry, DownloadKey, DownloadStatus, DownloadsController};
use jitter_core::models::{Quality, VideoId};
use jitter_core::playback::PlaybackTarget;
use jitter_twitch::Twitch;
use rinf::{DartSignal, DartSignalPack, RustSignal, SignalReceiver};
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;

use crate::format::twitch_error;
use crate::run::Fetched;
use crate::signals::{
    CancelDownload, ChooseDownload, DownloadItem, DownloadState, DownloadsReady, DownloadsUpdate,
    QualityOption, RemoveDownload, StartDownload,
};

/// Requests per segment before the download gives up on it.
const ATTEMPTS: u32 = 3;
/// The pause between two requests for one segment.
const RETRY_PAUSE: Duration = Duration::from_secs(1);

/// The downloads: the controller's list, and the writer task of each one
/// still running.
pub struct DownloadsPanel {
    controller: DownloadsController,
    running: HashMap<DownloadKey, Running>,
}

/// A running download's writer task, and the part file it writes into,
/// which a cancel deletes. Dropping it aborts the task.
struct Running {
    task: JoinHandle<()>,
    part: PathBuf,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// A result of a download's tasks on its way back into the loop, tagged
/// with what it was for.
pub enum DownloadFetched {
    /// A VOD's qualities, for `ChooseDownload`.
    Choices {
        video_id: String,
        result: Result<Vec<Quality>, String>,
    },
    /// A VOD resolved for `StartDownload`, to be written into `dir`.
    Resolved {
        video: VideoId,
        dir: PathBuf,
        result: Result<Download, String>,
    },
    /// The writer joined another segment.
    Progress {
        key: DownloadKey,
        progress: DownloadProgress,
    },
    /// The writer ended: the final file's path, or why it stopped. Its
    /// part file is kept when it stopped.
    Ended {
        key: DownloadKey,
        result: Result<String, String>,
    },
}

impl DownloadsPanel {
    pub fn new() -> Self {
        Self {
            controller: DownloadsController::new(),
            running: HashMap::new(),
        }
    }
}

/// One Dart signal about downloads, with its message.
pub enum DownloadIntent {
    Ready(DownloadsReady),
    Choose(ChooseDownload),
    Start(StartDownload),
    Cancel(CancelDownload),
    Remove(RemoveDownload),
}

/// The five download signals' receivers, taken once. The hub's main
/// `select!` sits at tokio's 64-branch limit, so these reach it through one
/// arm: `next` picks whichever signal arrives first.
pub struct DownloadsInbox {
    ready: SignalReceiver<DartSignalPack<DownloadsReady>>,
    choose: SignalReceiver<DartSignalPack<ChooseDownload>>,
    start: SignalReceiver<DartSignalPack<StartDownload>>,
    cancel: SignalReceiver<DartSignalPack<CancelDownload>>,
    remove: SignalReceiver<DartSignalPack<RemoveDownload>>,
}

impl DownloadsInbox {
    /// Takes the receivers. rinf keeps one per signal type, so call this once.
    pub fn new() -> Self {
        Self {
            ready: DownloadsReady::get_dart_signal_receiver(),
            choose: ChooseDownload::get_dart_signal_receiver(),
            start: StartDownload::get_dart_signal_receiver(),
            cancel: CancelDownload::get_dart_signal_receiver(),
            remove: RemoveDownload::get_dart_signal_receiver(),
        }
    }

    /// The next download signal, or None once all five are closed.
    pub async fn next(&mut self) -> Option<DownloadIntent> {
        tokio::select! {
            Some(pack) = self.ready.recv() => Some(DownloadIntent::Ready(pack.message)),
            Some(pack) = self.choose.recv() => Some(DownloadIntent::Choose(pack.message)),
            Some(pack) = self.start.recv() => Some(DownloadIntent::Start(pack.message)),
            Some(pack) = self.cancel.recv() => Some(DownloadIntent::Cancel(pack.message)),
            Some(pack) = self.remove.recv() => Some(DownloadIntent::Remove(pack.message)),
            else => None,
        }
    }
}

/// Runs one download signal against the list. Starting and choosing only
/// spawn calls, so they need no panel.
pub fn handle(
    panel: &mut DownloadsPanel,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
    intent: DownloadIntent,
) {
    match intent {
        DownloadIntent::Ready(_) => ready(panel),
        DownloadIntent::Choose(signal) => choose(twitch, fetch, signal),
        DownloadIntent::Start(signal) => start(twitch, fetch, signal),
        DownloadIntent::Cancel(signal) => cancel(panel, signal),
        DownloadIntent::Remove(signal) => remove(panel, signal),
    }
}

/// `DownloadsReady`: the whole list, for a holder that just subscribed.
pub fn ready(panel: &DownloadsPanel) {
    send_snapshot(panel);
}

/// `ChooseDownload`: loads the VOD's qualities. `on_fetched` sends them as
/// `Choices`, or the failure as `Failed`.
pub fn choose(twitch: &Twitch, fetch: &UnboundedSender<Fetched>, signal: ChooseDownload) {
    let video = match parse_video(&signal.video_id) {
        Ok(video) => video,
        Err(message) => return failed(message),
    };
    let (twitch, fetch) = (twitch.clone(), fetch.clone());
    tokio::spawn(async move {
        let result = twitch
            .vod(&video)
            .await
            .map(|playback| playback.qualities)
            .map_err(|e| twitch_error(&e));
        let _ = fetch.send(Fetched::Downloads(DownloadFetched::Choices {
            video_id: signal.video_id,
            result,
        }));
    });
}

/// `StartDownload`: resolves the VOD for the chosen quality. `on_fetched`
/// starts the writer once it has resolved.
pub fn start(twitch: &Twitch, fetch: &UnboundedSender<Fetched>, signal: StartDownload) {
    let video = match parse_video(&signal.video_id) {
        Ok(video) => video,
        Err(message) => return failed(message),
    };
    let dir = PathBuf::from(signal.dir);
    let quality = signal.quality;
    let (twitch, fetch) = (twitch.clone(), fetch.clone());
    tokio::spawn(async move {
        let result = resolve(&twitch, &video, &quality).await;
        let _ = fetch.send(Fetched::Downloads(DownloadFetched::Resolved {
            video,
            dir,
            result,
        }));
    });
}

/// `CancelDownload`: stops a running download and deletes its part file.
pub fn cancel(panel: &mut DownloadsPanel, signal: CancelDownload) {
    let key = DownloadKey::new(signal.key);
    if !panel.controller.cancel(key) {
        return;
    }
    if let Some(running) = panel.running.remove(&key) {
        running.task.abort();
        // Synchronous, unlike the writes: a new download of the same file
        // must not create its part file before this delete has run.
        if let Err(e) = std::fs::remove_file(&running.part) {
            log::warn!("removing {}: {e}", running.part.display());
        }
    }
    send_snapshot(panel);
}

/// `RemoveDownload`: drops a download that is no longer running from the list.
pub fn remove(panel: &mut DownloadsPanel, signal: RemoveDownload) {
    if panel.controller.remove(DownloadKey::new(signal.key)) {
        send_snapshot(panel);
    }
}

/// A download task's result, handed to the list; then the list goes out
/// where it changed. A resolved download starts its writer.
pub fn on_fetched(
    panel: &mut DownloadsPanel,
    fetched: DownloadFetched,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
) {
    match fetched {
        DownloadFetched::Choices { video_id, result } => match result {
            Ok(qualities) => DownloadsUpdate::Choices {
                video_id,
                qualities: qualities.iter().map(QualityOption::from).collect(),
            }
            .send_signal_to_dart(),
            Err(e) => failed(format!("Couldn't load the qualities: {e}")),
        },
        DownloadFetched::Resolved { video, dir, result } => match result {
            Ok(download) => start_writer(panel, video, dir, download, twitch, fetch),
            Err(message) => failed(message),
        },
        DownloadFetched::Progress { key, progress } => {
            if panel.controller.progress(key, progress) {
                send_snapshot(panel);
            }
        }
        DownloadFetched::Ended { key, result } => {
            // Dropping the entry aborts its task, which is ending anyway.
            panel.running.remove(&key);
            match result {
                Ok(path) => panel.controller.finish(key, path),
                Err(message) => panel.controller.fail(key, message),
            };
            send_snapshot(panel);
        }
    }
}

/// Starts a resolved download, unless its file is already in the directory
/// or already being written. Both are refused before any byte is written.
fn start_writer(
    panel: &mut DownloadsPanel,
    video: VideoId,
    dir: PathBuf,
    download: Download,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
) {
    let done = dir.join(download.file_name());
    if done.exists() {
        return failed(format!("{} already exists", done.display()));
    }
    let Some(key) = panel.controller.start(video, &download) else {
        return failed(format!("already downloading {}", download.file_name()));
    };
    let part = dir.join(download.part_name());
    let task = spawn_writer(twitch, fetch, key, download, dir);
    panel.running.insert(key, Running { task, part });
    send_snapshot(panel);
}

/// Resolves a VOD for one quality: what a download needs before its file
/// can be named and started.
async fn resolve(twitch: &Twitch, video: &VideoId, name: &str) -> Result<Download, String> {
    let target = PlaybackTarget::Vod(video.clone());
    let (playback, details) = tokio::join!(twitch.vod(video), twitch.playback_details(&target));
    let playback =
        playback.map_err(|e| format!("Couldn't start the download: {}", twitch_error(&e)))?;
    // Without the channel, the file is named by the VOD's id alone.
    let login = match details {
        Ok(details) => details.channel.map(|channel| channel.login),
        Err(e) => {
            log::warn!(
                "channel unknown, naming the file by id: {}",
                twitch_error(&e)
            );
            None
        }
    };
    let Some(quality) = playback.quality(name) else {
        let names: Vec<&str> = playback.qualities.iter().map(|q| q.name.as_str()).collect();
        return Err(format!(
            "no quality named {name:?}; have: {}",
            names.join(", ")
        ));
    };
    let list = twitch
        .segments(&quality.url)
        .await
        .map_err(|e| format!("Couldn't read the segment list: {}", twitch_error(&e)))?;
    Ok(Download::new(login.as_ref(), video, &quality.name, list))
}

/// Writes one download in its own task. Its progress and its end come back
/// as `Fetched`; an abort leaves no result behind.
fn spawn_writer(
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
    key: DownloadKey,
    mut download: Download,
    dir: PathBuf,
) -> JoinHandle<()> {
    let (twitch, fetch) = (twitch.clone(), fetch.clone());
    tokio::spawn(async move {
        let result = write(&twitch, &fetch, key, &mut download, &dir).await;
        // The loop is gone: the app is shutting down.
        let _ = fetch.send(Fetched::Downloads(DownloadFetched::Ended { key, result }));
    })
}

/// Appends each segment to `<name>.part` in `dir`, in order, then renames
/// the part file to the final name and returns that name's path. A failure
/// returns before the rename and leaves the part file where it is.
async fn write(
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
    key: DownloadKey,
    download: &mut Download,
    dir: &Path,
) -> Result<String, String> {
    let part = dir.join(download.part_name());
    let done = dir.join(download.file_name());
    let create = part.clone();
    let mut file = blocking(move || File::create(create))
        .await
        .map_err(|e| format!("creating {}: {e}", part.display()))?;
    // Cloned so that `record` can borrow the download mutably.
    let urls = download.urls().to_vec();
    let total = urls.len();
    for (index, url) in urls.iter().enumerate() {
        let bytes = fetch_segment(twitch, url, index + 1, total).await?;
        let len = bytes.len();
        let (next, ()) = on_file(file, move |out| out.write_all(&bytes))
            .await
            .map_err(|e| format!("writing {}: {e}", part.display()))?;
        file = next;
        download.record(len);
        let progress = download.progress();
        if fetch
            .send(Fetched::Downloads(DownloadFetched::Progress {
                key,
                progress,
            }))
            .is_err()
        {
            return Err("the app is shutting down".to_owned());
        }
    }
    let (out, ()) = on_file(file, |out| out.sync_all())
        .await
        .map_err(|e| format!("syncing {}: {e}", part.display()))?;
    // Closed before the rename: Windows refuses to rename an open file.
    drop(out);
    let (from, to) = (part.clone(), done.clone());
    blocking(move || std::fs::rename(from, to))
        .await
        .map_err(|e| format!("renaming {} to {}: {e}", part.display(), done.display()))?;
    Ok(done.display().to_string())
}

/// One segment's bytes. A failed request is tried again, up to `ATTEMPTS` in
/// all, because one transient error should not lose a download of ~1000
/// segments.
async fn fetch_segment(
    twitch: &Twitch,
    url: &str,
    n: usize,
    total: usize,
) -> Result<Vec<u8>, String> {
    let mut attempt = 1;
    loop {
        match twitch.segment(url).await {
            Ok(bytes) => return Ok(bytes),
            Err(e) if attempt == ATTEMPTS => {
                return Err(format!("segment {n} of {total}: {}", twitch_error(&e)));
            }
            Err(_) => {
                attempt += 1;
                tokio::time::sleep(RETRY_PAUSE).await;
            }
        }
    }
}

/// Runs `op` on a blocking thread, so a slow disk never stalls the hub's
/// runtime.
async fn blocking<T: Send + 'static>(
    op: impl FnOnce() -> io::Result<T> + Send + 'static,
) -> io::Result<T> {
    tokio::task::spawn_blocking(op)
        .await
        .map_err(io::Error::other)?
}

/// Runs `op` on `file` on a blocking thread, and hands the file back for
/// the next step.
async fn on_file<T: Send + 'static>(
    mut file: File,
    op: impl FnOnce(&mut File) -> io::Result<T> + Send + 'static,
) -> io::Result<(File, T)> {
    let (file, result) = tokio::task::spawn_blocking(move || {
        let result = op(&mut file);
        (file, result)
    })
    .await
    .map_err(io::Error::other)?;
    Ok((file, result?))
}

/// Sends the whole list: it replaces Dart's.
fn send_snapshot(panel: &DownloadsPanel) {
    DownloadsUpdate::Snapshot {
        downloads: panel
            .controller
            .entries()
            .iter()
            .map(DownloadItem::from)
            .collect(),
    }
    .send_signal_to_dart();
}

/// Tells Dart a request or a start failed: a one-shot notice, not state.
fn failed(message: String) {
    DownloadsUpdate::Failed { message }.send_signal_to_dart();
}

/// The VOD id Dart sent, or a message saying it is not one.
fn parse_video(input: &str) -> Result<VideoId, String> {
    VideoId::parse(input).map_err(|_| format!("{input:?} is not a VOD id"))
}

impl From<&DownloadEntry> for DownloadItem {
    fn from(entry: &DownloadEntry) -> Self {
        let state = match &entry.status {
            DownloadStatus::Running => DownloadState::Running,
            DownloadStatus::Done { path } => DownloadState::Done { path: path.clone() },
            DownloadStatus::Failed { message } => DownloadState::Failed {
                message: message.clone(),
            },
            DownloadStatus::Cancelled => DownloadState::Cancelled,
        };
        Self {
            key: entry.key.get(),
            video_id: entry.video.as_str().to_owned(),
            file_name: entry.file_name.clone(),
            done: count(entry.progress.done),
            total: count(entry.progress.total),
            bytes: entry.progress.bytes,
            state,
        }
    }
}

/// A count as the list shows it: one past `u32::MAX` reads as `u32::MAX`.
fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use jitter_core::models::SegmentList;

    use super::*;

    type TestResult = Result<(), String>;

    /// A VOD of two segments, named without a channel.
    fn resolved(id: &str) -> Result<(VideoId, Download), String> {
        let video = VideoId::parse(id).map_err(|e| format!("{e:?}"))?;
        let list = SegmentList {
            init: None,
            segments: vec!["https://cdn/1.ts".to_owned(), "https://cdn/2.ts".to_owned()],
        };
        let download = Download::new(None, &video, "720p60", list);
        Ok((video, download))
    }

    /// The list as Dart sees it.
    fn items(controller: &DownloadsController) -> Vec<DownloadItem> {
        controller
            .entries()
            .iter()
            .map(DownloadItem::from)
            .collect()
    }

    #[test]
    fn an_entry_maps_to_an_item() -> TestResult {
        let mut controller = DownloadsController::new();
        let (video, mut download) = resolved("2600000000")?;
        let key = controller.start(video, &download).ok_or("no key")?;
        download.record(5);
        controller.progress(key, download.progress());

        let items = items(&controller);
        let [item] = items.as_slice() else {
            return Err(format!("expected one item, got {}", items.len()));
        };
        assert_eq!(item.key, key.get());
        assert_eq!(item.video_id, "2600000000");
        assert_eq!(item.file_name, download.file_name());
        assert_eq!((item.done, item.total, item.bytes), (1, 2, 5));
        assert!(matches!(item.state, DownloadState::Running));
        Ok(())
    }

    #[test]
    fn ended_states_map_with_their_detail() -> TestResult {
        let mut controller = DownloadsController::new();
        let mut keys = Vec::new();
        for id in ["1", "2", "3"] {
            let (video, download) = resolved(id)?;
            keys.push(controller.start(video, &download).ok_or("no key")?);
        }
        let [finished, broken, stopped] = keys.as_slice() else {
            return Err("expected three keys".to_owned());
        };
        controller.finish(*finished, "/out/1_720p60.ts".to_owned());
        controller.fail(*broken, "segment 2 of 2: HTTP 404".to_owned());
        controller.cancel(*stopped);

        let items = items(&controller);
        let [done, failed, cancelled] = items.as_slice() else {
            return Err("expected three items".to_owned());
        };
        let DownloadState::Done { path } = &done.state else {
            return Err("the first item is not done".to_owned());
        };
        assert_eq!(path, "/out/1_720p60.ts");
        let DownloadState::Failed { message } = &failed.state else {
            return Err("the second item is not failed".to_owned());
        };
        assert_eq!(message, "segment 2 of 2: HTTP 404");
        assert!(matches!(cancelled.state, DownloadState::Cancelled));
        Ok(())
    }
}
