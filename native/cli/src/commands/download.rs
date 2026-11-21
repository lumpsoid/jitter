use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use clap::Args;
use jitter_core::download::Download;
use jitter_core::models::Playback;
use jitter_core::playback::PlaybackTarget;
use jitter_twitch::Twitch;

use super::{Result, ctx, twitch, video_id};

/// Requests per segment before the download gives up on it.
const ATTEMPTS: u32 = 3;
/// The pause between two requests for one segment.
const RETRY_PAUSE: Duration = Duration::from_secs(1);

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
/// existing file with the final name is an error, never overwritten. On any
/// error the `.part` file is left where it is: nothing is cleaned up, and the
/// next run truncates it and starts over.
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
    let mut file =
        File::create(&part_path).map_err(ctx(format!("creating {}", part_path.display())))?;
    // Cloned so that `record` can borrow the download mutably.
    let urls = download.urls().to_vec();
    let total = urls.len();
    for (index, url) in urls.iter().enumerate() {
        let bytes = fetch_segment(&twitch, url, index + 1, total).await?;
        file.write_all(&bytes)
            .map_err(ctx(format!("writing {}", part_path.display())))?;
        download.record(bytes.len());
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
    // Best effort, as the progress line is: a closed stderr must not fail a finished download.
    let _ = writeln!(std::io::stderr());
    println!("{}", done_path.display());
    Ok(())
}

/// One segment's bytes. A failed request is tried again, up to `ATTEMPTS` in
/// all, because one transient error should not lose a download of ~1000 segments.
async fn fetch_segment(twitch: &Twitch, url: &str, n: usize, total: usize) -> Result<Vec<u8>> {
    let mut attempt = 1;
    loop {
        match twitch.segment(url).await {
            Ok(bytes) => return Ok(bytes),
            Err(e) if attempt == ATTEMPTS => {
                return Err(ctx(format!("segment {n} of {total}"))(e));
            }
            Err(_) => {
                attempt += 1;
                tokio::time::sleep(RETRY_PAUSE).await;
            }
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
