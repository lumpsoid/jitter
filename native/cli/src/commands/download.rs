use clap::Args;
use futures_util::StreamExt;
use jitter_core::download::Download;
use jitter_core::models::Playback;
use jitter_core::playback::PlaybackTarget;
use jitter_twitch::part_file::{self, Rebase};
use std::io::Write;
use std::path::PathBuf;

use super::{Result, ctx, twitch, video_id};

#[derive(Args)]
pub struct DownloadArgs {
    /// VOD id, as `jitter vods` lists it.
    vod: String,
    /// Quality name (for example `720p60`); `source` is the best one.
    #[arg(long, default_value = Playback::DEFAULT_QUALITY)]
    quality: String,
    /// Directory to write into; the current one when left out.
    #[arg(long, value_name = "DIR")]
    out: Option<PathBuf>,
}

/// Joins a VOD quality's segments, in order, into one file that core names.
///
/// The bytes go to `<name>.part`, which is renamed to the final name only once
/// every segment is in, so the final name never holds a partial file. An
/// existing file with the final name is an error, never overwritten.
///
/// An fMP4 VOD's timestamps are moved to start at zero as they are written,
/// so players show its real length.
///
/// After each segment, `<name>.part.resume` records how much of the `.part`
/// file is whole. On any error, or a Ctrl-C, both files stay where they are,
/// and the next run of the same download carries on after the last whole
/// segment. Without a checkpoint that fits, it truncates and starts over.
pub async fn run(args: DownloadArgs) -> Result<()> {
    let twitch = twitch()?;
    let id = video_id(&args.vod)?;
    let target = PlaybackTarget::Vod(id.clone());
    let (playback, details) = tokio::join!(twitch.vod(&id), twitch.playback_details(&target));
    let playback = playback.map_err(ctx("loading the VOD"))?;
    let login = match details {
        Ok(details) => details.channel.map(|channel| channel.login),
        Err(e) => {
            eprintln!("-- channel unknown, naming the file by id: {e}");
            None
        }
    };
    let Some(quality) = playback.quality(&args.quality) else {
        let names: Vec<&str> = playback.qualities.iter().map(|q| q.name.as_str()).collect();
        return Err(format!(
            "no quality named {:?}; have: {}",
            args.quality,
            names.join(", ")
        ));
    };
    let list = twitch
        .segments(&quality.url)
        .await
        .map_err(ctx("reading the segment list"))?;
    let mut download = Download::new(login.as_ref(), &id, &quality.name, list);

    let dir = args.out.unwrap_or_else(|| PathBuf::from("."));
    let done_path = dir.join(download.file_name());
    if done_path.exists() {
        return Err(format!("{} already exists", done_path.display()));
    }
    let part_path = dir.join(download.part_name());
    let checkpoint_path = dir.join(download.checkpoint_name());
    let mut file = part_file::open(&part_path, &checkpoint_path, &mut download)
        .map_err(ctx(format!("opening {}", part_path.display())))?;
    let resumed = download.progress().done;
    if resumed > 0 {
        eprintln!("-- resuming {}", part_path.display());
        progress(&download);
    }
    let total = download.progress().total;
    // Cloned so that `record` can borrow the download mutably.
    let urls = download.remaining().to_vec();
    let mut segments = std::pin::pin!(twitch.segment_bytes(&urls).enumerate());
    let mut rebase = Rebase::new();
    while let Some((index, bytes)) = segments.next().await {
        let n = resumed + index + 1;
        let mut bytes = bytes.map_err(ctx(format!("segment {n} of {total}")))?;
        rebase.prepare(&mut download, &mut bytes);
        file.write_all(&bytes)
            .map_err(ctx(format!("writing {}", part_path.display())))?;
        download.record(bytes.len());
        // Only after the bytes it counts are in the part file.
        std::fs::write(&checkpoint_path, download.checkpoint())
            .map_err(ctx(format!("writing {}", checkpoint_path.display())))?;
        progress(&download);
    }
    file.sync_all()
        .map_err(ctx(format!("syncing {}", part_path.display())))?;
    drop(file);
    std::fs::rename(&part_path, &done_path).map_err(ctx(format!(
        "renaming {} to {}",
        part_path.display(),
        done_path.display()
    )))?;
    if let Err(e) = std::fs::remove_file(&checkpoint_path) {
        eprintln!("-- removing {}: {e}", checkpoint_path.display());
    }
    // Best effort, as the progress line is: a closed stderr must not fail a finished download.
    let _ = writeln!(std::io::stderr());
    println!("{}", done_path.display());
    Ok(())
}

/// `12/1000 parts, 340.2 MiB`, redrawn over the previous line.
fn progress(download: &Download) {
    let p = download.progress();
    let mib = p.bytes as f64 / 1_048_576.0;
    let mut stderr = std::io::stderr();
    // Best effort: a closed stderr must not abort the download.
    let _ = write!(stderr, "\r{}/{} parts, {mib:.1} MiB", p.done, p.total);
    let _ = stderr.flush();
}
