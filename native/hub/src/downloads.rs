//! The downloads list: download signals ↔ the downloads controller, and one
//! writer task per running download, which fetches the VOD's segments and
//! joins them into one file.
//!
//! A download is resolved before it starts (playback, channel, segment list),
//! so the controller refuses a file that is already being written before any
//! byte is written. File writes run on tokio's blocking threads: the hub's
//! runtime is single-threaded and also carries every UI signal.
//!
//! A cancel does not abort the writer: an aborted task leaves its blocking
//! file op running, which could create or rename a file after the cancel
//! had cleaned up. The writer is told to stop, stops between file ops, and
//! removes its own files before it ends.
//!
//! The steps follow the CLI's `jitter download`, which this copies rather
//! than imports.

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use futures_util::StreamExt;
use jitter_core::download::{Download, DownloadProgress};
use jitter_core::downloads::{DownloadEntry, DownloadKey, DownloadStatus, DownloadsController};
use jitter_core::models::{Quality, VideoId};
use jitter_core::playback::PlaybackTarget;
use jitter_twitch::Twitch;
use jitter_twitch::part_file::{self, Rebase};
use rinf::{DartSignal, DartSignalPack, RustSignal, SignalReceiver};
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::Instant;

use crate::format::twitch_error;
use crate::run::Fetched;
use crate::signals::{
    CancelDownload, ChooseDownload, DownloadItem, DownloadState, DownloadsReady, DownloadsUpdate,
    QualityOption, RemoveDownload, StartDownload,
};

/// The least time between two progress updates of one download. Each one
/// sends Dart the whole list, and a fast link joins many segments a second;
/// a count redrawn twice a second reads the same.
const PROGRESS_EVERY: Duration = Duration::from_millis(500);

/// The downloads: the controller's list, and the writer task of each one
/// still running.
pub struct DownloadsPanel {
    controller: DownloadsController,
    running: HashMap<DownloadKey, Running>,
}

/// A download's writer task, from its start until it ends, even when
/// cancelled: the part file it writes into is not free until then.
/// Dropping it aborts the task, which only the app's shutdown does early.
struct Running {
    task: JoinHandle<()>,
    stop: watch::Sender<bool>,
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
    /// The writer ended and its files are settled.
    Ended { key: DownloadKey, ending: Ending },
}

/// How a writer ended.
pub enum Ending {
    /// The final file's path.
    Done(String),
    /// Why it failed. The part file is kept.
    Failed(String),
    /// It was told to stop, and removed what it had written.
    Cancelled,
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
        Err(message) => return failed(signal.video_id, message),
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
        Err(message) => return failed(signal.video_id, message),
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

/// `CancelDownload`: tells a running download's writer to stop. The writer
/// deletes its part file itself, once no file op of its is in flight.
pub fn cancel(panel: &mut DownloadsPanel, signal: CancelDownload) {
    let key = DownloadKey::new(signal.key);
    if !panel.controller.cancel(key) {
        return;
    }
    if let Some(running) = panel.running.get(&key) {
        running.stop.send_replace(true);
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
            Err(e) => failed(video_id, format!("Couldn't load the qualities: {e}")),
        },
        DownloadFetched::Resolved { video, dir, result } => match result {
            Ok(download) => start_writer(panel, video, dir, download, twitch, fetch),
            Err(message) => failed(video.as_str().to_owned(), message),
        },
        DownloadFetched::Progress { key, progress } => {
            if panel.controller.progress(key, progress) {
                send_snapshot(panel);
            }
        }
        DownloadFetched::Ended { key, ending } => {
            // Dropping the entry aborts its task, which is ending anyway.
            panel.running.remove(&key);
            let changed = match ending {
                Ending::Done(path) => panel.controller.finish(key, path),
                Ending::Failed(message) => panel.controller.fail(key, message),
                // The cancel already marked it.
                Ending::Cancelled => false,
            };
            if changed {
                send_snapshot(panel);
            }
        }
    }
}

/// Starts a resolved download, unless its file is already in the directory,
/// already being written, or still being cleaned up after a cancel. All
/// three are refused before any byte is written. Answers the start with
/// `Started` or `Failed`.
fn start_writer(
    panel: &mut DownloadsPanel,
    video: VideoId,
    dir: PathBuf,
    download: Download,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
) {
    let id = video.as_str().to_owned();
    let done = dir.join(download.file_name());
    if done.exists() {
        return failed(id, format!("{} already exists", done.display()));
    }
    let part = dir.join(download.part_name());
    if panel.running.values().any(|running| running.part == part) {
        return failed(id, format!("still stopping {}", download.file_name()));
    }
    let Some(key) = panel.controller.start(video, &download) else {
        return failed(id, format!("already downloading {}", download.file_name()));
    };
    let (stop, stopped) = watch::channel(false);
    let task = spawn_writer(twitch, fetch, key, download, dir, stopped);
    panel.running.insert(key, Running { task, stop, part });
    send_snapshot(panel);
    DownloadsUpdate::Started { video_id: id }.send_signal_to_dart();
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
    mut stop: watch::Receiver<bool>,
) -> JoinHandle<()> {
    let (twitch, fetch) = (twitch.clone(), fetch.clone());
    tokio::spawn(async move {
        let files = Files::new(&dir, &download);
        let ending = match write(&twitch, &fetch, key, &mut download, &files, &mut stop).await {
            Ok(path) => Ending::Done(path),
            Err(Stop::Failed(message)) => Ending::Failed(message),
            Err(Stop::Cancelled) => {
                files.discard().await;
                Ending::Cancelled
            }
        };
        // The loop is gone: the app is shutting down.
        let _ = fetch.send(Fetched::Downloads(DownloadFetched::Ended { key, ending }));
    })
}

/// Why `write` returned early.
enum Stop {
    Failed(String),
    Cancelled,
}

/// The paths one download writes in its directory.
struct Files {
    part: PathBuf,
    checkpoint: PathBuf,
    done: PathBuf,
}

impl Files {
    fn new(dir: &Path, download: &Download) -> Self {
        Self {
            part: dir.join(download.part_name()),
            checkpoint: dir.join(download.checkpoint_name()),
            done: dir.join(download.file_name()),
        }
    }

    /// Removes the part file and its checkpoint, for a cancelled download.
    async fn discard(&self) {
        remove_if_there(&self.part).await;
        remove_if_there(&self.checkpoint).await;
    }
}

/// Removes `path`, logging a failure. A missing file is fine: the writer may
/// have stopped before creating it.
async fn remove_if_there(path: &Path) {
    let owned = path.to_owned();
    match blocking(move || std::fs::remove_file(owned)).await {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => log::warn!("removing {}: {e}", path.display()),
    }
}

/// Opens the part file where `download` picks up: after an earlier run's
/// bytes when its checkpoint still fits them, else empty.
async fn open_part(download: &mut Download, files: &Files) -> io::Result<File> {
    let (part, checkpoint) = (files.part.clone(), files.checkpoint.clone());
    let mut resumed = download.clone();
    let (resumed, file) = blocking(move || {
        let file = part_file::open(&part, &checkpoint, &mut resumed)?;
        Ok((resumed, file))
    })
    .await?;
    *download = resumed;
    Ok(file)
}

/// Appends each segment to the part file, in order, an fMP4 one's
/// timestamps moved to start at zero, then renames it to the final name and
/// returns that name's path. A failure returns before the
/// rename and leaves the part file and its checkpoint where they are, for
/// the next start of the same download to resume.
///
/// `stop` is checked after every file op, never during one, so nothing this
/// started is still touching the disk when it returns `Cancelled`.
async fn write(
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
    key: DownloadKey,
    download: &mut Download,
    files: &Files,
    stop: &mut watch::Receiver<bool>,
) -> Result<String, Stop> {
    let Files {
        part,
        checkpoint,
        done,
    } = files;
    let mut file = open_part(download, files)
        .await
        .map_err(|e| failed_to(format!("opening {}: {e}", part.display())))?;
    stopped(stop)?;
    let DownloadProgress {
        done: resumed,
        total,
        ..
    } = download.progress();
    let mut last_sent: Option<Instant> = None;
    if resumed > 0 {
        last_sent = Some(Instant::now());
        send_progress(fetch, key, download)?;
    }
    // Cloned so that `record` can borrow the download mutably.
    let urls = download.remaining().to_vec();
    // Dropping the stream on a return drops its requests in flight.
    let mut segments = std::pin::pin!(twitch.segment_bytes(&urls).enumerate());
    let mut rebase = Rebase::new();
    loop {
        let next = tokio::select! {
            next = segments.next() => next,
            _ = stop.wait_for(|stop| *stop) => return Err(Stop::Cancelled),
        };
        let Some((index, bytes)) = next else {
            break;
        };
        let mut bytes = bytes.map_err(|e| {
            failed_to(format!(
                "segment {} of {total}: {}",
                resumed + index + 1,
                twitch_error(&e)
            ))
        })?;
        rebase.prepare(download, &mut bytes);
        download.record(bytes.len());
        let (saved, to) = (download.checkpoint(), checkpoint.clone());
        // The checkpoint only after the bytes it counts are in the part file.
        let (next, ()) = on_file(file, move |out| {
            out.write_all(&bytes)?;
            std::fs::write(to, saved)
        })
        .await
        .map_err(|e| failed_to(format!("writing {}: {e}", part.display())))?;
        file = next;
        stopped(stop)?;
        let due = last_sent.is_none_or(|at| at.elapsed() >= PROGRESS_EVERY);
        // The last count always goes out, so the list ends on it.
        if !due && !download.is_finished() {
            continue;
        }
        last_sent = Some(Instant::now());
        send_progress(fetch, key, download)?;
    }
    let (out, ()) = on_file(file, |out| out.sync_all())
        .await
        .map_err(|e| failed_to(format!("syncing {}: {e}", part.display())))?;
    // Closed before the rename: Windows refuses to rename an open file.
    drop(out);
    stopped(stop)?;
    let (from, to) = (part.clone(), done.clone());
    blocking(move || std::fs::rename(from, to))
        .await
        .map_err(|e| {
            failed_to(format!(
                "renaming {} to {}: {e}",
                part.display(),
                done.display()
            ))
        })?;
    if stopped(stop).is_err() {
        // The cancel landed during the rename. The list already shows the
        // download as cancelled, so the file it finished goes too.
        let path = done.clone();
        if let Err(e) = blocking(move || std::fs::remove_file(path)).await {
            log::warn!("removing {}: {e}", done.display());
        }
        return Err(Stop::Cancelled);
    }
    remove_if_there(checkpoint).await;
    Ok(done.display().to_string())
}

fn send_progress(
    fetch: &UnboundedSender<Fetched>,
    key: DownloadKey,
    download: &Download,
) -> Result<(), Stop> {
    let progress = download.progress();
    fetch
        .send(Fetched::Downloads(DownloadFetched::Progress {
            key,
            progress,
        }))
        .map_err(|_| failed_to("the app is shutting down"))
}

/// `Cancelled` once the download was told to stop, or its panel is gone.
fn stopped(stop: &watch::Receiver<bool>) -> Result<(), Stop> {
    match stop.has_changed() {
        Ok(_) if !*stop.borrow() => Ok(()),
        _ => Err(Stop::Cancelled),
    }
}

fn failed_to(message: impl Into<String>) -> Stop {
    Stop::Failed(message.into())
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

/// Tells Dart a request or a start for `video_id` failed: a one-shot
/// notice, not state.
fn failed(video_id: String, message: String) {
    DownloadsUpdate::Failed { video_id, message }.send_signal_to_dart();
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
    use std::sync::{Arc, Mutex};

    use jitter_core::models::SegmentList;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    use tokio::sync::mpsc::{self, UnboundedReceiver};

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

    /// What the stand-in CDN does for one path.
    #[derive(Clone)]
    enum Serve {
        Bytes(&'static [u8]),
        /// Holds the request open and never answers.
        Hang,
    }

    /// A stand-in for the CDN on a local port: serves each path as `routes`
    /// says, 404 for any other, and records every path asked for.
    struct Cdn {
        base: String,
        asked: Arc<Mutex<Vec<String>>>,
    }

    impl Cdn {
        async fn start(routes: &[(&str, Serve)]) -> Result<Self, String> {
            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .map_err(|e| e.to_string())?;
            let base = format!(
                "http://{}",
                listener.local_addr().map_err(|e| e.to_string())?
            );
            let routes: HashMap<String, Serve> = routes
                .iter()
                .map(|(path, serve)| ((*path).to_owned(), serve.clone()))
                .collect();
            let asked = Arc::new(Mutex::new(Vec::new()));
            let log = Arc::clone(&asked);
            tokio::spawn(async move {
                while let Ok((conn, _)) = listener.accept().await {
                    tokio::spawn(answer(conn, routes.clone(), Arc::clone(&log)));
                }
            });
            Ok(Self { base, asked })
        }

        fn url(&self, path: &str) -> String {
            format!("{}{path}", self.base)
        }

        fn asked(&self) -> Vec<String> {
            self.asked
                .lock()
                .map(|asked| asked.clone())
                .unwrap_or_default()
        }
    }

    /// Answers one request on `conn`, then closes it.
    async fn answer(
        mut conn: TcpStream,
        routes: HashMap<String, Serve>,
        asked: Arc<Mutex<Vec<String>>>,
    ) {
        let mut head = Vec::new();
        let mut buf = [0u8; 1024];
        while !head.windows(4).any(|w| w == b"\r\n\r\n") {
            match conn.read(&mut buf).await {
                Ok(0) | Err(_) => return,
                Ok(n) => head.extend_from_slice(&buf[..n]),
            }
        }
        let head = String::from_utf8_lossy(&head);
        let path = head.split(' ').nth(1).unwrap_or("").to_owned();
        if let Ok(mut asked) = asked.lock() {
            asked.push(path.clone());
        }
        let (status, body): (&str, &[u8]) = match routes.get(&path) {
            Some(Serve::Bytes(body)) => ("200 OK", body),
            Some(Serve::Hang) => return std::future::pending().await,
            None => ("404 Not Found", b""),
        };
        let reply = format!(
            "HTTP/1.1 {status}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
            body.len()
        );
        let _ = conn.write_all(reply.as_bytes()).await;
        let _ = conn.write_all(body).await;
    }

    /// A download of `paths` on `cdn`, named without a channel.
    fn download_of(cdn: &Cdn, paths: &[&str]) -> Result<Download, String> {
        let video = VideoId::parse("42").map_err(|e| format!("{e:?}"))?;
        let list = SegmentList {
            init: None,
            segments: paths.iter().map(|path| cdn.url(path)).collect(),
        };
        Ok(Download::new(None, &video, "480p", list))
    }

    /// The next thing the writer sends, failing the test after a while
    /// rather than hanging it.
    async fn next_fetched(
        fetched: &mut UnboundedReceiver<Fetched>,
    ) -> Result<DownloadFetched, String> {
        match tokio::time::timeout(Duration::from_secs(20), fetched.recv()).await {
            Ok(Some(Fetched::Downloads(fetched))) => Ok(fetched),
            Ok(Some(_)) => Err("the writer sent something else".to_owned()),
            Ok(None) => Err("the writer's channel closed".to_owned()),
            Err(_) => Err("the writer sent nothing for 20 s".to_owned()),
        }
    }

    /// How the writer ended, skipping its progress.
    async fn ending(fetched: &mut UnboundedReceiver<Fetched>) -> Result<Ending, String> {
        loop {
            if let DownloadFetched::Ended { ending, .. } = next_fetched(fetched).await? {
                return Ok(ending);
            }
        }
    }

    #[tokio::test]
    async fn a_cancel_mid_download_removes_the_part_file_and_its_checkpoint() -> TestResult {
        let cdn = Cdn::start(&[("/0.ts", Serve::Bytes(b"aaaa")), ("/1.ts", Serve::Hang)]).await?;
        let download = download_of(&cdn, &["/0.ts", "/1.ts"])?;
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let files = Files::new(dir.path(), &download);
        let twitch = Twitch::new().map_err(|e| twitch_error(&e))?;
        let (fetch, mut fetched) = mpsc::unbounded_channel();
        let (stop, stopped) = watch::channel(false);
        let key = DownloadKey::new(1);
        let _task = spawn_writer(
            &twitch,
            &fetch,
            key,
            download,
            dir.path().to_owned(),
            stopped,
        );

        // The first segment is in; the second never comes.
        let DownloadFetched::Progress { progress, .. } = next_fetched(&mut fetched).await? else {
            return Err("expected progress first".to_owned());
        };
        assert_eq!(progress.done, 1);
        assert!(files.part.exists() && files.checkpoint.exists());

        stop.send_replace(true);
        assert!(matches!(ending(&mut fetched).await?, Ending::Cancelled));
        assert!(!files.part.exists(), "the part file is left");
        assert!(!files.checkpoint.exists(), "the checkpoint is left");
        assert!(!files.done.exists());
        Ok(())
    }

    #[tokio::test]
    async fn a_restart_resumes_after_the_last_whole_segment() -> TestResult {
        // The first segment would fail the download if it were fetched again.
        let cdn = Cdn::start(&[
            ("/1.ts", Serve::Bytes(b"BBBB")),
            ("/2.ts", Serve::Bytes(b"CC")),
        ])
        .await?;
        let paths = ["/0.ts", "/1.ts", "/2.ts"];
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let mut earlier = download_of(&cdn, &paths)?;
        let files = Files::new(dir.path(), &earlier);
        earlier.record(4);
        // An earlier run wrote the first segment and half of the second.
        std::fs::write(&files.checkpoint, earlier.checkpoint()).map_err(|e| e.to_string())?;
        std::fs::write(&files.part, b"aaaabb").map_err(|e| e.to_string())?;

        let twitch = Twitch::new().map_err(|e| twitch_error(&e))?;
        let (fetch, mut fetched) = mpsc::unbounded_channel();
        let (_stop, stopped) = watch::channel(false);
        let download = download_of(&cdn, &paths)?;
        let _task = spawn_writer(
            &twitch,
            &fetch,
            DownloadKey::new(1),
            download,
            dir.path().to_owned(),
            stopped,
        );

        let Ending::Done(path) = ending(&mut fetched).await? else {
            return Err("the resumed download did not finish".to_owned());
        };
        assert_eq!(path, files.done.display().to_string());
        assert_eq!(
            std::fs::read(&files.done).map_err(|e| e.to_string())?,
            b"aaaaBBBBCC"
        );
        assert!(!files.part.exists() && !files.checkpoint.exists());
        assert_eq!(cdn.asked(), ["/1.ts", "/2.ts"]);
        Ok(())
    }
}
