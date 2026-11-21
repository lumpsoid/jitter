use clap::Args;
use futures_util::StreamExt;
use jitter_core::download::{Download, PartStart};
use jitter_core::models::Playback;
use jitter_core::playback::PlaybackTarget;
use std::fs::File;
use std::io::{self, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

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
    let start = match saved_run(&part_path, &checkpoint_path) {
        Some((saved, part_len)) => download.resume(&saved, part_len),
        None => PartStart::Fresh,
    };
    let mut file =
        open(&part_path, start).map_err(ctx(format!("opening {}", part_path.display())))?;
    let resumed = download.progress().done;
    if resumed > 0 {
        eprintln!("-- resuming {}", part_path.display());
        progress(&download);
    }
    let total = download.progress().total;
    // Cloned so that `record` can borrow the download mutably.
    let urls = download.remaining().to_vec();
    let mut segments = std::pin::pin!(twitch.segment_bytes(&urls).enumerate());
    while let Some((index, bytes)) = segments.next().await {
        let n = resumed + index + 1;
        let bytes = bytes.map_err(ctx(format!("segment {n} of {total}")))?;
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

/// The checkpoint's text and the part file's length, when both are there.
fn saved_run(part: &Path, checkpoint: &Path) -> Option<(String, u64)> {
    let saved = std::fs::read_to_string(checkpoint).ok()?;
    let part_len = std::fs::metadata(part).ok()?.len();
    Some((saved, part_len))
}

/// The part file, opened for `start`'s next byte.
fn open(part: &Path, start: PartStart) -> io::Result<File> {
    match start {
        PartStart::Fresh => File::create(part),
        PartStart::Resume { keep } => {
            let mut file = File::options().write(true).open(part)?;
            file.set_len(keep)?;
            file.seek(SeekFrom::End(0))?;
            Ok(file)
        }
    }
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
