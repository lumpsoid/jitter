use clap::Subcommand;
use jitter_core::favorites::FavoritesController;
use jitter_core::models::Favorite;

use super::{Result, ctx, login, now, open_store, twitch};

#[derive(Subcommand)]
pub enum FavCommand {
    /// Look a channel up and store it as a favorite.
    Add { login: String },
    /// Remove a stored favorite by login (no network).
    Rm { login: String },
}

pub async fn run(command: Option<FavCommand>) -> Result<()> {
    let mut favorites = FavoritesController::new(Box::new(open_store()?));
    favorites.load().map_err(ctx("loading favorites"))?;
    match command {
        Some(FavCommand::Add { login: input }) => {
            let login = login(&input)?;
            let channel = twitch()?
                .channel(&login)
                .await
                .map_err(ctx(format!("looking up {login}")))?;
            let name = channel.display_name.clone();
            favorites
                .add(channel, now()?)
                .map_err(ctx("saving the favorite"))?;
            println!("added {name}");
            Ok(())
        }
        Some(FavCommand::Rm { login: input }) => {
            let login = login(&input)?;
            let id = favorites
                .find_by_login(&login)
                .ok_or_else(|| format!("{login} is not a favorite"))?;
            favorites
                .remove(&id)
                .map_err(ctx("removing the favorite"))?;
            println!("removed {login}");
            Ok(())
        }
        None => list(&mut favorites).await,
    }
}

async fn list(favorites: &mut FavoritesController) -> Result<()> {
    let streams = twitch()?
        .live_streams(&favorites.ids())
        .await
        .map_err(ctx("checking live status"))?;
    favorites
        .apply_live(streams)
        .map_err(ctx("updating favorites"))?;
    let now = now()?;
    let split = favorites.split();
    for stream in &split.live {
        let uptime = stream.uptime(now).map_or_else(
            || "?".to_owned(),
            |up| format!("{}h{:02}m", up.get() / 3600, up.get() / 60 % 60),
        );
        println!(
            "live     {:>8}  {uptime:>7}  {:<20} {}",
            stream.viewers, stream.channel.login, stream.title
        );
    }
    print_idle("offline", &split.offline);
    print_idle("unknown", &split.unchecked);
    Ok(())
}

fn print_idle(status: &str, favorites: &[&Favorite]) {
    for favorite in favorites {
        println!("{status:<8} {:>8}  {:>7}  {}", "", "", favorite.login);
    }
}
