use std::sync::OnceLock;
use std::time::Instant;

use futures_util::StreamExt;
use jitter_core::chat::ChatController;
use jitter_core::models::{ChatEvent, ChatMessage, EmoteScope, Fragment};
use jitter_twitch::emotes::ProviderEmotes;

use super::{Result, ctx, login, twitch};

/// Prints a channel's live chat until interrupted, emotes as `[name]`.
/// `verbose` also prints the connection's own log (PING/PONG, reconnects)
/// to stderr.
pub async fn run(input: &str, verbose: bool) -> Result<()> {
    if verbose {
        StderrLog::install()?;
    }
    let login = login(input)?;
    let twitch = twitch()?;
    let channel = twitch
        .channel(&login)
        .await
        .map_err(ctx(format!("looking up {login}")))?;

    let mut chat = ChatController::new();
    let (global, local) = tokio::join!(twitch.global_emotes(), twitch.channel_emotes(&channel.id));
    load_emotes(&mut chat, EmoteScope::Global, global);
    load_emotes(&mut chat, EmoteScope::Channel, local);

    let events = jitter_twitch::chat::connect(login.clone());
    let mut events = std::pin::pin!(events);
    while let Some(event) = events.next().await {
        print_event(&chat, &event, &login.to_string());
        // A terminal is append-only: the CLI prints events, not diffs.
        chat.apply(event);
    }
    Ok(())
}

fn load_emotes(chat: &mut ChatController, scope: EmoteScope, results: [ProviderEmotes; 3]) {
    for (provider, result) in results {
        match result {
            // The CLI renders each message once, on arrival; nothing to redraw.
            Ok(emotes) => {
                chat.set_emotes(scope, provider, emotes);
            }
            Err(e) => {
                eprintln!("-- {provider:?} {scope:?} emotes unavailable: {e}");
                chat.provider_failed(provider);
            }
        }
    }
}

fn print_event(chat: &ChatController, event: &ChatEvent, channel: &str) {
    match event {
        ChatEvent::Message(message) => println!("{}", render(chat, message)),
        ChatEvent::Connected => println!("-- connected to #{channel}"),
        ChatEvent::Disconnected => println!("-- disconnected; reconnecting"),
        ChatEvent::Notice(text) => println!("-- {text}"),
        ChatEvent::ClearAll => println!("-- chat cleared"),
        ChatEvent::ClearMessage { .. } | ChatEvent::ClearUser { .. } | ChatEvent::RoomState(_) => {}
    }
}

/// `12:34:56 name: text [Emote] text`, or `12:34:56 * name text` for /me.
pub(super) fn render(chat: &ChatController, message: &ChatMessage) -> String {
    let body: String = chat
        .fragments(message)
        .into_iter()
        .map(|fragment| match fragment {
            Fragment::Text(text) | Fragment::Link(text) => text,
            Fragment::Emote(emote) => format!("[{}]", emote.name),
            Fragment::Overlay { base, overlays } => {
                let names: Vec<&str> = overlays.iter().map(|e| e.name.as_str()).collect();
                format!("[{}+{}]", base.name, names.join("+"))
            }
        })
        .collect();
    let seconds = message.sent_at.unix().rem_euclid(86_400);
    let time = format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    );
    let name = &message.sender.display_name;
    if message.action {
        format!("{time} * {name} {body}")
    } else {
        format!("{time} {name}: {body}")
    }
}

/// The smallest logger that shows the connection's debug lines, with the
/// seconds since start.
struct StderrLog {
    start: Instant,
}

static LOGGER: OnceLock<StderrLog> = OnceLock::new();

impl StderrLog {
    fn install() -> Result<()> {
        let logger = LOGGER.get_or_init(|| StderrLog {
            start: Instant::now(),
        });
        log::set_logger(logger).map_err(ctx("installing the logger"))?;
        log::set_max_level(log::LevelFilter::Debug);
        Ok(())
    }
}

impl log::Log for StderrLog {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.target().starts_with("jitter")
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            eprintln!(
                "[{:>7.1}s] {}",
                self.start.elapsed().as_secs_f64(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}
