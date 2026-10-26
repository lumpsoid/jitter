use jitter_core::models::Seconds;
use jitter_core::resume::ResumeController;
use jitter_core::resume_policy::ResumeDecision;

use super::{Result, ctx, now, open_store, video_id};

pub fn run(id: &str, position_s: u32, duration_s: u32) -> Result<()> {
    let mut resume = ResumeController::new(Box::new(open_store()?));
    let decision = resume
        .record(
            video_id(id)?,
            Seconds::new(position_s),
            Seconds::new(duration_s),
            now()?,
        )
        .map_err(ctx("recording the position"))?;
    println!(
        "{}",
        match decision {
            ResumeDecision::Save => "saved",
            ResumeDecision::Clear => "cleared (finished)",
            ResumeDecision::Skip => "skipped (unchanged)",
        }
    );
    Ok(())
}
