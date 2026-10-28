use chrono::{Local, TimeZone};
use clap::Subcommand;
use jitter_core::history::HistoryController;
use jitter_core::models::{Visit, VisitId};
use jitter_core::playback::PlaybackTarget;

use super::{Result, ctx, open_store};

#[derive(Subcommand)]
pub enum HistoryCommand {
    /// List each live channel and VOD once, by its latest visit.
    Unique,
    /// Remove one visit by its id (the first column).
    Rm { id: i64 },
    /// Remove every visit.
    Clear,
}

pub fn run(command: Option<HistoryCommand>) -> Result<()> {
    let mut history = HistoryController::new(Box::new(open_store()?));
    history.load().map_err(ctx("loading the history"))?;
    match command {
        None => {
            history.visits().iter().for_each(print);
            Ok(())
        }
        Some(HistoryCommand::Unique) => {
            history.unique().into_iter().for_each(print);
            Ok(())
        }
        Some(HistoryCommand::Rm { id }) => {
            history
                .remove(VisitId::from_raw(id))
                .map_err(ctx("removing the visit"))?;
            println!("removed visit {id}");
            Ok(())
        }
        Some(HistoryCommand::Clear) => {
            history.clear().map_err(ctx("clearing the history"))?;
            println!("cleared the history");
            Ok(())
        }
    }
}

fn print(visit: &Visit) {
    let started = Local
        .timestamp_opt(visit.started_at.unix(), 0)
        .single()
        .map_or_else(
            || "?".to_owned(),
            |at| at.format("%Y-%m-%d %H:%M").to_string(),
        );
    let watched = visit.watched().get();
    let (kind, target) = match &visit.target {
        PlaybackTarget::Live(login) => ("live", login.as_str()),
        PlaybackTarget::Vod(id) => ("vod", id.as_str()),
    };
    let title = visit.details.title.as_deref().unwrap_or("");
    println!(
        "{:>6}  {started}  {:>2}h{:02}m  {kind:<4} {target:<20} {title}",
        visit.id.get(),
        watched / 3600,
        watched / 60 % 60,
    );
}
