//! The session log: one file per app run in `logs/` beside the database,
//! named by when the run started; the newest [`KEEP`] are kept. Rust's
//! `log` records, Dart's `LogFromDart` lines and the player's buffering
//! waits all land in it, so a user can attach one file to an issue. The
//! logs page lists and reads the files.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use chrono::Local;
use log::{Level, LevelFilter, Log, Metadata, Record};
use rinf::RustSignal;

use crate::signals::{DartLogLevel, LogFromDart, LogSession, LogsUpdate, OpenLog, PlayerBuffering};

/// Sessions kept, this one included.
pub const KEEP: usize = 10;

const EXTENSION: &str = "log";

/// This run's log, installed as the `log` logger.
pub struct SessionLog {
    dir: PathBuf,
    /// This run's file name.
    current: String,
}

impl SessionLog {
    /// Removes all but the newest `KEEP - 1` sessions in `dir`, then opens
    /// this run's file and installs it as the `log` logger. Once per
    /// process: `log` takes one logger.
    pub fn start(dir: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        let unremoved: Vec<String> = stale(sessions_in(&dir)?, KEEP - 1)
            .into_iter()
            .filter_map(|name| {
                fs::remove_file(dir.join(&name))
                    .err()
                    .map(|e| format!("{name}: {e}"))
            })
            .collect();
        let current = format!("{}.{EXTENSION}", Local::now().format("%Y-%m-%d_%H-%M-%S"));
        let path = dir.join(&current);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| format!("cannot open {}: {e}", path.display()))?;
        log::set_boxed_logger(Box::new(FileLogger {
            file: Mutex::new(file),
        }))
        .map_err(|e| format!("cannot start the log: {e}"))?;
        log::set_max_level(LevelFilter::Debug);
        log::info!(
            "jitter {} on {} {}",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH,
        );
        for failure in unremoved {
            log::warn!("removing an old log: {failure}");
        }
        Ok(Self { dir, current })
    }
}

/// This run's log, or why it did not start; the logs page shows the
/// reason in place of the sessions.
pub type Started = Result<SessionLog, String>;

/// `LogsReady`: the kept sessions, newest first.
pub fn ready(log: &Started) {
    match log.as_ref().map_err(|e| not_started(e)).and_then(sessions) {
        Ok(update) => update,
        Err(message) => LogsUpdate::Failed { message },
    }
    .send_signal_to_dart();
}

fn sessions(log: &SessionLog) -> Result<LogsUpdate, String> {
    Ok(LogsUpdate::Sessions {
        dir: log.dir.display().to_string(),
        sessions: sessions_in(&log.dir)?
            .into_iter()
            .rev()
            .map(|name| LogSession {
                current: name == log.current,
                label: label(&name),
                name,
            })
            .collect(),
    })
}

/// `OpenLog`: the session's whole text. Only a kept session's, so a name
/// cannot reach outside the directory.
pub fn open(log: &Started, signal: OpenLog) {
    let read = log
        .as_ref()
        .map_err(|e| not_started(e))
        .and_then(|log| read(&log.dir, &signal.name));
    match read {
        Ok((path, text)) => LogsUpdate::Opened {
            name: signal.name,
            path: path.display().to_string(),
            text,
        },
        Err(message) => LogsUpdate::Failed { message },
    }
    .send_signal_to_dart();
}

/// When session `name` started, as the page shows it: `2026-11-03 14:22:05`.
fn label(name: &str) -> String {
    let stem = name.trim_end_matches(&format!(".{EXTENSION}")).to_owned();
    match stem.split_once('_') {
        Some((day, time)) => format!("{day} {}", time.replace('-', ":")),
        None => stem,
    }
}

fn not_started(reason: &str) -> String {
    format!("Logging did not start: {reason}")
}

fn read(dir: &Path, name: &str) -> Result<(PathBuf, String), String> {
    if !sessions_in(dir)?.iter().any(|kept| kept == name) {
        return Err(format!("No log named {name}"));
    }
    let path = dir.join(name);
    let bytes = fs::read(&path).map_err(|e| format!("Could not read {name}: {e}"))?;
    Ok((path, String::from_utf8_lossy(&bytes).into_owned()))
}

/// `LogFromDart`: one line, under the source Dart gave.
pub fn from_dart(signal: LogFromDart) {
    let level = match signal.level {
        DartLogLevel::Error => Level::Error,
        DartLogLevel::Warning => Level::Warn,
        DartLogLevel::Info => Level::Info,
    };
    log::log!(target: "dart", level, "{}: {}", signal.source, signal.message);
}

/// When the player began waiting for data, while it waits.
#[derive(Default)]
pub struct Buffering {
    since: Option<Instant>,
}

/// `PlayerBuffering`: logs a wait's start, with how much was buffered
/// ahead (none means the network fell behind), and its length at its end.
pub fn buffering(waiting: &mut Buffering, signal: PlayerBuffering, now: Instant) {
    if let Some(line) = waiting.report(&signal, now) {
        log::info!("{line}");
    }
}

impl Buffering {
    /// The line for this report; none for a repeat of the last one.
    fn report(&mut self, signal: &PlayerBuffering, now: Instant) -> Option<String> {
        match (signal.buffering, self.since) {
            (true, None) => {
                self.since = Some(now);
                let ahead = signal.buffer_ms.saturating_sub(signal.position_ms);
                Some(format!(
                    "buffering at {:.1} s, {:.1} s buffered ahead",
                    seconds(signal.position_ms),
                    seconds(ahead),
                ))
            }
            (false, Some(since)) => {
                self.since = None;
                let waited = now.saturating_duration_since(since).as_secs_f64();
                Some(format!("buffering ended after {waited:.1} s"))
            }
            _ => None,
        }
    }
}

fn seconds(ms: u32) -> f64 {
    f64::from(ms) / 1000.0
}

/// Writes each record as one line to this run's file, and to stderr for a
/// developer's terminal.
struct FileLogger {
    file: Mutex<File>,
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= level_for(metadata.target())
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = format!(
            "{} {:<5} {}: {}\n",
            Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
            record.level(),
            record.target(),
            without_queries(&record.args().to_string()),
        );
        eprint!("{line}");
        // A line that cannot be written is lost: there is nowhere to report it.
        if let Ok(mut file) = self.file.lock() {
            let _ = file.write_all(line.as_bytes());
        }
    }

    fn flush(&self) {
        if let Ok(mut file) = self.file.lock() {
            let _ = file.flush();
        }
    }
}

/// `text` with every URL's query cut to `?…`. Users share their logs,
/// and a playlist URL's query holds Twitch's access token; mpv's and
/// reqwest's errors quote the URL whole.
fn without_queries(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = ["https://", "http://"]
        .iter()
        .filter_map(|scheme| rest.find(scheme))
        .min()
    {
        let url_len = rest[start..]
            .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | ')' | '>'))
            .unwrap_or(rest.len() - start);
        let url = &rest[start..start + url_len];
        out.push_str(&rest[..start]);
        match url.split_once('?') {
            Some((bare, _)) => {
                out.push_str(bare);
                out.push_str("?…");
            }
            None => out.push_str(url),
        }
        rest = &rest[start + url_len..];
    }
    out.push_str(rest);
    out
}

/// The most detailed level logged from `target`: ours from debug, other
/// crates' (HTTP, TLS) from warnings, as their debug lines would drown ours.
fn level_for(target: &str) -> Level {
    let ours = ["hub", "jitter_", "dart"]
        .iter()
        .any(|prefix| target.starts_with(prefix));
    if ours { Level::Debug } else { Level::Warn }
}

/// The kept sessions' names, oldest first: they sort by start time.
fn sessions_in(dir: &Path) -> Result<Vec<String>, String> {
    let entries =
        fs::read_dir(dir).map_err(|e| format!("Could not list {}: {e}", dir.display()))?;
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == EXTENSION))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    names.sort();
    Ok(names)
}

/// The sessions to remove so that the newest `keep` remain.
fn stale(mut names: Vec<String>, keep: usize) -> Vec<String> {
    names.sort();
    names.truncate(names.len().saturating_sub(keep));
    names
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn names(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn a_url_s_query_is_cut_and_the_rest_kept() {
        let line = "error sending request for url \
            (https://usher.ttvnw.net/vod/1.m3u8?sig=a&token=b): timed out; \
            then 'http://edge.test/x' and https://edge.test/y?t=1";
        assert_eq!(
            without_queries(line),
            "error sending request for url (https://usher.ttvnw.net/vod/1.m3u8?…): \
             timed out; then 'http://edge.test/x' and https://edge.test/y?…"
        );
    }

    #[test]
    fn a_session_is_shown_by_when_it_started() {
        assert_eq!(label("2026-11-03_14-22-05.log"), "2026-11-03 14:22:05");
    }

    #[test]
    fn the_oldest_sessions_go_once_more_than_kept() {
        let kept = names(&[
            "2026-11-03_10-00-00.log",
            "2026-11-01_10-00-00.log",
            "2026-11-02_10-00-00.log",
        ]);
        assert_eq!(stale(kept, 2), names(&["2026-11-01_10-00-00.log"]));
    }

    #[test]
    fn no_session_goes_while_fewer_are_kept() {
        assert!(stale(names(&["2026-11-01_10-00-00.log"]), 2).is_empty());
    }

    fn report(buffering: bool, position_ms: u32, buffer_ms: u32) -> PlayerBuffering {
        PlayerBuffering {
            buffering,
            position_ms,
            buffer_ms,
        }
    }

    #[test]
    fn a_wait_logs_its_start_with_what_was_buffered_and_its_length_at_its_end() {
        let mut waiting = Buffering::default();
        let start = Instant::now();

        let began = waiting.report(&report(true, 61_000, 61_500), start);
        let ended = waiting.report(
            &report(false, 61_000, 64_000),
            start + Duration::from_millis(2_340),
        );

        assert_eq!(
            began.as_deref(),
            Some("buffering at 61.0 s, 0.5 s buffered ahead")
        );
        assert_eq!(ended.as_deref(), Some("buffering ended after 2.3 s"));
    }

    #[test]
    fn a_repeated_report_logs_nothing() {
        let mut waiting = Buffering::default();
        let now = Instant::now();

        assert!(waiting.report(&report(false, 0, 0), now).is_none());
        assert!(waiting.report(&report(true, 0, 0), now).is_some());
        assert!(waiting.report(&report(true, 0, 0), now).is_none());
    }
}
