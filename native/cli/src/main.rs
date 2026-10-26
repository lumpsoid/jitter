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
    /// Manage favorite channels; without a subcommand, list them with live status.
    Fav {
        #[command(subcommand)]
        command: Option<FavCommand>,
    },
    /// List a channel's past broadcasts.
    Vods {
        login: String,
        /// How many pages to load.
        #[arg(long, default_value_t = 1)]
        pages: u32,
    },
    /// Resolve a live channel or VOD and print its qualities or one URL.
    Play(PlayArgs),
    /// Print a channel's live chat, emotes as `[name]`, until interrupted.
    Chat {
        login: String,
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
        Command::Fav { command } => commands::fav::run(command).await,
        Command::Vods { login, pages } => commands::vods::run(&login, pages).await,
        Command::Play(args) => commands::play::run(args).await,
        Command::Chat { login, verbose } => commands::chat::run(&login, verbose).await,
        Command::Resume {
            id,
            position_s,
            duration_s,
        } => commands::resume::run(&id, position_s, duration_s),
    }
}
