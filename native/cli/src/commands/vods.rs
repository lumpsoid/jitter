use jitter_core::models::VideoKind;

use super::{Result, ctx, login, twitch};

pub async fn run(login_input: &str, pages: u32) -> Result<()> {
    let login = login(login_input)?;
    let twitch = twitch()?;
    let mut cursor = None;
    for _ in 0..pages {
        let page = twitch
            .channel_videos(&login, VideoKind::Archive, cursor.as_ref())
            .await
            .map_err(ctx(format!("loading {login}'s VODs")))?;
        for video in &page.items {
            let seconds = video.duration.get();
            println!(
                "{:<12} {:>3}:{:02}:{:02}  {}",
                video.id,
                seconds / 3600,
                seconds / 60 % 60,
                seconds % 60,
                video.title
            );
        }
        cursor = page.next;
        if cursor.is_none() {
            break;
        }
    }
    Ok(())
}
