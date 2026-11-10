//! History controller ↔ history signals. The player's position reports
//! reach the controller from the playback arms in `run.rs`; core decides
//! what they record. The list goes out whole when it changes, and once
//! more when a player is left, with the visit's final time; a removal's
//! failure goes out first as a one-shot `Failed`. A recording failure is
//! only logged: the player keeps playing and the next report tries again.

use std::fmt::Display;

use chrono::NaiveDate;
use jitter_core::history::{HistoryChange, HistoryController};
use jitter_core::models::{Visit, VisitId};
use jitter_core::playback::{self, PlaybackController};
use rinf::RustSignal;

use crate::format::{
    clock_label, day_label, day_time_label, local_time, streamed_label, today, watched_label,
};
use crate::playback::now;
use crate::signals::{
    HistoryDay, HistoryEntry, HistoryUpdate, PlaybackChannel, PlaybackTarget, RemoveFromHistory,
    RemoveVisit,
};

/// `HistoryReady`: the history as it stands.
pub fn ready(history: &HistoryController) {
    send_snapshot(history);
}

/// A position report core took for `target` (the playback arm's
/// `PlayerAt`): core decides whether it is watching, and the list goes
/// out when a visit was added or carried on.
pub fn heard(
    history: &mut HistoryController,
    playback: &PlaybackController,
    target: &playback::PlaybackTarget,
) {
    let change = now().and_then(|now| {
        history
            .heard(playback, target, now)
            .map_err(|e| format!("recording the history: {e}"))
    });
    match change {
        Ok(HistoryChange::Listed) => send_snapshot(history),
        Ok(HistoryChange::Unchanged | HistoryChange::Touched) => {}
        Err(e) => log::warn!("{e}"),
    }
}

/// `LeavePlayer`, before the player leaves: the visit's final time is
/// written and the list goes out with it. An invalid target is the
/// playback arm's to report.
pub fn leave(
    history: &mut HistoryController,
    playback: &PlaybackController,
    target: &PlaybackTarget,
) {
    let Ok(target) = playback::PlaybackTarget::try_from(target.clone()) else {
        return;
    };
    let change = now().and_then(|now| {
        history
            .leave(playback, &target, now)
            .map_err(|e| format!("recording the history: {e}"))
    });
    match change {
        Ok(HistoryChange::Unchanged) => {}
        Ok(HistoryChange::Touched | HistoryChange::Listed) => send_snapshot(history),
        Err(e) => log::warn!("{e}"),
    }
}

/// `RemoveVisit`.
pub fn remove(history: &mut HistoryController, signal: RemoveVisit) {
    report(
        history.remove(VisitId::from_raw(signal.id)),
        "Couldn't remove the visit",
    );
    send_snapshot(history);
}

/// `RemoveFromHistory`.
pub fn remove_target(history: &mut HistoryController, signal: RemoveFromHistory) {
    let removed = playback::PlaybackTarget::try_from(signal.target)
        .and_then(|target| history.remove_target(&target).map_err(|e| e.to_string()));
    report(removed, "Couldn't remove it from the history");
    send_snapshot(history);
}

/// `ClearHistory`.
pub fn clear(history: &mut HistoryController) {
    report(history.clear(), "Couldn't clear the history");
    send_snapshot(history);
}

/// Re-reads the store after an import, then sends the history.
pub fn reload(history: &mut HistoryController) {
    report(history.load(), "Couldn't reload the history");
    send_snapshot(history);
}

/// Sends a failed action's error as `Failed`, prefixed with `what`.
fn report<E: Display>(result: Result<(), E>, what: &str) {
    if let Err(e) = result {
        HistoryUpdate::Failed {
            message: format!("{what}: {e}"),
        }
        .send_signal_to_dart();
    }
}

fn send_snapshot(history: &HistoryController) {
    snapshot(history, today()).send_signal_to_dart();
}

/// The history as sent, its days named as seen on `today`.
fn snapshot(history: &HistoryController, today: NaiveDate) -> HistoryUpdate {
    let mut days: Vec<HistoryDay> = Vec::new();
    for visit in history.visits() {
        let started = local_time(visit.started_at);
        let label = started.map_or_else(
            || "Unknown day".to_owned(),
            |at| day_label(at.date(), today),
        );
        let entry = entry(visit, started.map(clock_label));
        match days.last_mut() {
            Some(day) if day.label == label => day.entries.push(entry),
            _ => days.push(HistoryDay {
                label,
                entries: vec![entry],
            }),
        }
    }
    let unique = history
        .unique()
        .into_iter()
        .map(|visit| {
            let when = local_time(visit.started_at).map(|at| day_time_label(at, today));
            entry(visit, when)
        })
        .collect();
    HistoryUpdate::Snapshot { days, unique }
}

/// `when` is `None` for a start chrono cannot place.
fn entry(visit: &Visit, when: Option<String>) -> HistoryEntry {
    let details = &visit.details;
    HistoryEntry {
        id: visit.id.get(),
        target: PlaybackTarget::from(&visit.target),
        live: matches!(visit.target, playback::PlaybackTarget::Live(_)),
        title: title(visit),
        channel: details.channel.as_ref().map(PlaybackChannel::from),
        game: details.game.as_ref().map(|game| game.name.clone()),
        when: when.unwrap_or_else(|| "Unknown time".to_owned()),
        watched: watched_label(visit.watched()),
        streamed: details.streamed_at.and_then(streamed_label),
    }
}

/// The title, else the channel's name, else what the target names.
fn title(visit: &Visit) -> String {
    let details = &visit.details;
    details
        .title
        .clone()
        .or_else(|| details.channel.as_ref().map(|c| c.display_name.clone()))
        .unwrap_or_else(|| match &visit.target {
            playback::PlaybackTarget::Live(login) => login.as_str().to_owned(),
            playback::PlaybackTarget::Vod(id) => format!("VOD {}", id.as_str()),
        })
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use jitter_core::fakes::FakeHistoryStore;
    use jitter_core::models::{ChannelLogin, Timestamp, VideoId, VisitDetails};

    use super::*;

    fn visit(id: i64, target: playback::PlaybackTarget, started_at: i64) -> Visit {
        Visit {
            id: VisitId::from_raw(id),
            target,
            started_at: Timestamp::from_unix(started_at),
            last_seen_at: Timestamp::from_unix(started_at + 2_400),
            details: VisitDetails::default(),
        }
    }

    fn live(login: &str) -> Result<playback::PlaybackTarget, String> {
        ChannelLogin::parse(login)
            .map(playback::PlaybackTarget::Live)
            .map_err(|e| format!("{e:?}"))
    }

    fn vod(id: &str) -> Result<playback::PlaybackTarget, String> {
        VideoId::parse(id)
            .map(playback::PlaybackTarget::Vod)
            .map_err(|e| format!("{e:?}"))
    }

    /// The local midnight starting `date`, as a timestamp.
    fn midnight(date: NaiveDate) -> Result<i64, String> {
        let at = date.and_hms_opt(0, 0, 0).ok_or("no midnight")?;
        let local = chrono::Local
            .from_local_datetime(&at)
            .earliest()
            .ok_or("no local midnight")?;
        Ok(local.timestamp())
    }

    #[test]
    fn the_timeline_groups_visits_by_the_day_they_started() -> Result<(), String> {
        let today = NaiveDate::from_ymd_opt(2026, 10, 28).ok_or("bad date")?;
        let yesterday = today.pred_opt().ok_or("bad date")?;
        let (t, y) = (midnight(today)?, midnight(yesterday)?);
        let store = FakeHistoryStore::with_visits(vec![
            visit(1, live("a")?, y + 3_600),
            visit(2, vod("42")?, t + 7_200),
            visit(3, live("a")?, t + 3_600),
        ]);
        let mut history = HistoryController::new(Box::new(store));
        history.load().map_err(|e| e.to_string())?;
        let HistoryUpdate::Snapshot { days, unique } = snapshot(&history, today) else {
            return Err("not a snapshot".to_owned());
        };
        let shape: Vec<(&str, Vec<i64>)> = days
            .iter()
            .map(|d| (d.label.as_str(), d.entries.iter().map(|e| e.id).collect()))
            .collect();
        assert_eq!(shape, [("Today", vec![2, 3]), ("Yesterday", vec![1])]);
        let first = days
            .first()
            .and_then(|d| d.entries.first())
            .ok_or("no entry")?;
        assert_eq!(first.when, "02:00");
        assert_eq!(first.watched, "40m");
        assert!(!first.live);
        let unique: Vec<(i64, &str)> = unique.iter().map(|e| (e.id, e.when.as_str())).collect();
        assert_eq!(unique, [(2, "Today, 02:00"), (3, "Today, 01:00")]);
        Ok(())
    }

    #[test]
    fn an_entry_is_titled_by_the_title_then_the_channel_then_the_target() -> Result<(), String> {
        let mut titled = visit(1, live("xqc")?, 0);
        titled.details.title = Some("GTA RP".to_owned());
        assert_eq!(title(&titled), "GTA RP");
        assert_eq!(title(&visit(2, live("xqc")?, 0)), "xqc");
        assert_eq!(title(&visit(3, vod("42")?, 0)), "VOD 42");
        Ok(())
    }
}
