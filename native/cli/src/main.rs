//! CLI head on the same core and Twitch crates as the GUI. If it cannot do
//! something without Flutter, logic leaked into the hub or Dart.
//!
//! Every command is the head loop: ask core what to load, call twitch, hand
//! the result back, print the state. Decisions stay in core.

mod commands;

use std::process::ExitCode;

use clap::{Parser, Subcommand};
use commands::fav::FavCommand;
use commands::play::PlayArgs;

#[derive(Parser)]
#[command(name = "jitter", about = "Watch Twitch without an account")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List the most-watched live channels.
    Top {
        /// How many pages to load.
        #[arg(long, default_value_t = 1)]
        pages: u32,
    },
    /// List the categories your live favorites are streaming, then the
    /// most-watched categories.
    Categories {
        /// How many pages to load.
        #[arg(long, default_value_t = 1)]
        pages: u32,
    },
    /// List a category's live streams by id (see `categories`), your
    /// favorites first.
    Category {
        id: String,
        /// How many pages to load.
        #[arg(long, default_value_t = 1)]
        pages: u32,
    },
    /// Find channels and categories by name.
    Search {
        /// What to search for; several words are one search.
        #[arg(required = true)]
        text: Vec<String>,
        /// How many pages of each to load.
        #[arg(long, default_value_t = 1)]
        pages: u32,
    },
    /// Manage favorite channels; without a subcommand, list them with live status.
    Fav {
        #[command(subcommand)]
        command: Option<FavCommand>,
    },
    /// Print a channel's description and list its past broadcasts.
    Vods {
        login: String,
        /// How many pages to load.
        #[arg(long, default_value_t = 1)]
        pages: u32,
    },
    /// Resolve a live channel or VOD and print its qualities or one URL.
    Play(PlayArgs),
    /// Print a channel's live chat, emotes as `[name]`, until interrupted;
    /// or, with `--vod`, a VOD's chat replayed over a stretch of the video.
    Chat {
        /// Channel login whose live chat to print.
        #[arg(required_unless_present = "vod", conflicts_with = "vod")]
        login: Option<String>,
        /// Replay this VOD's chat instead.
        #[arg(long)]
        vod: Option<String>,
        /// With `--vod`: the position, in seconds, the player starts at.
        #[arg(long, requires = "vod", default_value_t = 0)]
        at: u32,
        /// With `--vod`: how many seconds of playback to replay from `--at`.
        #[arg(long = "for", requires = "vod", default_value_t = 60)]
        span: u32,
        /// Also print the connection's log (PING/PONG, reconnects) to stderr.
        #[arg(long, short)]
        verbose: bool,
    },
    /// Record a VOD position by hand.
    Resume {
        id: String,
        position_s: u32,
        duration_s: u32,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli.command).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

async fn run(command: Command) -> commands::Result<()> {
    match command {
        Command::Top { pages } => commands::top::run(pages).await,
        Command::Categories { pages } => commands::categories::run(pages).await,
        Command::Category { id, pages } => commands::category::run(&id, pages).await,
        Command::Search { text, pages } => commands::search::run(&text.join(" "), pages).await,
        Command::Fav { command } => commands::fav::run(command).await,
        Command::Vods { login, pages } => commands::vods::run(&login, pages).await,
        Command::Play(args) => commands::play::run(args).await,
        Command::Chat {
            login,
            vod,
            at,
            span,
            verbose,
        } => match (login, vod) {
            (_, Some(id)) => commands::replay::run(&id, at, span).await,
            (Some(login), None) => commands::chat::run(&login, verbose).await,
            (None, None) => Err("give a channel login or --vod <id>".to_owned()),
        },
        Command::Resume {
            id,
            position_s,
            duration_s,
        } => commands::resume::run(&id, position_s, duration_s),
    }
}
