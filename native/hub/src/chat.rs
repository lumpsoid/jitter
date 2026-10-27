//! The live chat panel: the chat connection and emote fetches for the
//! channel being played, the chat controller ↔ chat signals, and the
//! batching of appended messages on their way to Dart.
//!
//! Each `Play` of a live channel starts a session: a fresh controller and
//! the tasks feeding it, whose results come back as [`Fetched::Chat`]
//! tagged with the session. Ending the session aborts its tasks, and a
//! result already on its way from them is dropped by its tag, so only one
//! connection ever feeds the panel.

use std::time::Duration;

use futures_util::StreamExt;
use jitter_core::chat::{ChatController, Connection};
use jitter_core::models::{
    ChannelLogin, ChatChange, ChatEvent, ChatMessage, Emote, EmoteProvider, EmoteScope, Fragment,
    RoomState, Seconds,
};
use jitter_core::name_color::{Theme, name_color};
use jitter_core::playback::PlaybackTarget;
use jitter_twitch::Twitch;
use jitter_twitch::emotes::ProviderEmotes;
use rinf::{RustSignal, debug_print};
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;
use tokio::time::Instant;

use crate::run::Fetched;
use crate::signals::{ChatFragment, ChatLine, ChatStatus, ChatUpdate, EmoteImage};

/// Appended messages are gathered this long and sent as one `Appended`:
/// transport batching, so a busy channel (~8 messages a second) costs
/// Dart a rebuild per window instead of per message.
const BATCH_WINDOW: Duration = Duration::from_millis(100);

/// The hub's side of the chat panel.
pub struct ChatPanel {
    live: Option<LiveChat>,
    /// Sessions started so far; the latest one's tag.
    sessions: u64,
    theme: Theme,
    batch: Batch,
}

/// The chat of the live channel being played.
struct LiveChat {
    session: u64,
    chat: ChatController,
    _tasks: Tasks,
}

/// The connection and emote fetches of one session. Dropping it aborts
/// them.
struct Tasks(Vec<JoinHandle<()>>);

impl Drop for Tasks {
    fn drop(&mut self) {
        self.0.iter().for_each(JoinHandle::abort);
    }
}

/// Appended messages not yet sent, and the evictions that followed them.
/// Evicting from the oldest end commutes with appending at the newest, so
/// sending all appends and then one summed eviction ends in the same list.
#[derive(Default)]
struct Batch {
    messages: Vec<ChatLine>,
    evicted: usize,
    due: Option<Instant>,
}

/// A chat task's output, back in the loop as [`Fetched::Chat`].
pub struct ChatFetched {
    session: u64,
    payload: Payload,
}

enum Payload {
    Event(ChatEvent),
    Emotes {
        scope: EmoteScope,
        results: [(EmoteProvider, Result<Vec<Emote>, String>); 3],
    },
}

impl ChatPanel {
    pub fn new() -> Self {
        Self {
            live: None,
            sessions: 0,
            theme: Theme::Dark,
            batch: Batch::default(),
        }
    }
}

/// `ChatReady`: the panel as it stands, for a holder that just subscribed.
pub fn ready(panel: &mut ChatPanel) {
    send_snapshot(panel);
}

/// `SetTheme`: recolours every name for the new background.
pub fn set_theme(panel: &mut ChatPanel, dark: bool) {
    panel.theme = match dark {
        true => Theme::Dark,
        false => Theme::Light,
    };
    send_snapshot(panel);
}

/// A player page opened on `target` (`None`: core rejected it). A live
/// channel gets a new session; anything else ends the current one.
pub fn play(
    panel: &mut ChatPanel,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
    target: Option<&PlaybackTarget>,
) {
    match target {
        Some(PlaybackTarget::Live(login)) => start(panel, twitch, fetch, login),
        Some(PlaybackTarget::Vod(_)) | None => stop(panel),
    }
}

/// Ends the session, aborting its connection and fetches.
pub fn stop(panel: &mut ChatPanel) {
    panel.live = None;
    send_snapshot(panel);
}

fn start(
    panel: &mut ChatPanel,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
    login: &ChannelLogin,
) {
    panel.sessions += 1;
    let session = panel.sessions;
    let tasks = Tasks(vec![
        spawn_connection(session, login.clone(), fetch.clone()),
        spawn_global_emotes(session, twitch.clone(), fetch.clone()),
        spawn_channel_emotes(session, login.clone(), twitch.clone(), fetch.clone()),
    ]);
    // Replacing the session drops (and so aborts) the previous tasks.
    panel.live = Some(LiveChat {
        session,
        chat: ChatController::new(),
        _tasks: tasks,
    });
    send_snapshot(panel);
}

/// Forwards the connection's events, forever; it reconnects by itself.
fn spawn_connection(
    session: u64,
    login: ChannelLogin,
    fetch: UnboundedSender<Fetched>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut events = std::pin::pin!(jitter_twitch::chat::connect(login));
        while let Some(event) = events.next().await {
            let fetched = ChatFetched {
                session,
                payload: Payload::Event(event),
            };
            // The loop is gone: the app is shutting down.
            let Ok(()) = fetch.send(Fetched::Chat(fetched)) else {
                break;
            };
        }
    })
}

fn spawn_global_emotes(
    session: u64,
    twitch: Twitch,
    fetch: UnboundedSender<Fetched>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let results = with_messages(twitch.global_emotes().await);
        send_emotes(&fetch, session, EmoteScope::Global, results);
    })
}

/// The providers key channel emotes by channel id, so the channel is
/// looked up first; if that fails, every provider counts as failed.
fn spawn_channel_emotes(
    session: u64,
    login: ChannelLogin,
    twitch: Twitch,
    fetch: UnboundedSender<Fetched>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let results = match twitch.channel(&login).await {
            Ok(channel) => with_messages(twitch.channel_emotes(&channel.id).await),
            Err(e) => {
                let message = format!("looking up {login}: {e}");
                [
                    EmoteProvider::SevenTv,
                    EmoteProvider::Bttv,
                    EmoteProvider::Ffz,
                ]
                .map(|provider| (provider, Err(message.clone())))
            }
        };
        send_emotes(&fetch, session, EmoteScope::Channel, results);
    })
}

fn with_messages(results: [ProviderEmotes; 3]) -> [(EmoteProvider, Result<Vec<Emote>, String>); 3] {
    results.map(|(provider, result)| (provider, result.map_err(|e| e.to_string())))
}

fn send_emotes(
    fetch: &UnboundedSender<Fetched>,
    session: u64,
    scope: EmoteScope,
    results: [(EmoteProvider, Result<Vec<Emote>, String>); 3],
) {
    let fetched = ChatFetched {
        session,
        payload: Payload::Emotes { scope, results },
    };
    let _ = fetch.send(Fetched::Chat(fetched));
}

/// Hands a chat task's output to the controller and passes on what
/// changed. Output of an ended session is dropped.
pub fn on_fetched(panel: &mut ChatPanel, fetched: ChatFetched) {
    let ChatPanel {
        live, theme, batch, ..
    } = panel;
    let Some(live) = live.as_mut().filter(|l| l.session == fetched.session) else {
        return;
    };
    match fetched.payload {
        Payload::Event(event) => {
            let changes = live.chat.apply(event);
            pass_on(&live.chat, *theme, batch, changes);
        }
        Payload::Emotes { scope, results } => {
            for (provider, result) in results {
                match result {
                    Ok(emotes) => {
                        live.chat.set_emotes(scope, provider, emotes);
                    }
                    Err(e) => {
                        debug_print!("{provider:?} {scope:?} emotes unavailable: {e}");
                        live.chat.provider_failed(provider);
                    }
                }
            }
            // `set_emotes` always asks for a rerender and a failure changes
            // the hint, so one snapshot covers all three results.
            send_snapshot(panel);
        }
    }
}

/// Sends each change as its diff, in order. Appends and evictions wait in
/// the batch; anything else sends the batch first, so Dart sees every
/// change in the order core made it.
fn pass_on(chat: &ChatController, theme: Theme, batch: &mut Batch, changes: Vec<ChatChange>) {
    for change in changes {
        match change {
            ChatChange::Appended { message } => {
                batch.messages.push(line(chat, &message, theme));
                batch
                    .due
                    .get_or_insert_with(|| Instant::now() + BATCH_WINDOW);
            }
            ChatChange::Evicted { count } => batch.evicted += count,
            ChatChange::Marked { ids } => {
                send_batch(batch);
                ChatUpdate::Marked { ids }.send_signal_to_dart();
            }
            ChatChange::Cleared => {
                send_batch(batch);
                ChatUpdate::Cleared.send_signal_to_dart();
            }
            ChatChange::Rerender => {
                *batch = Batch::default();
                ChatUpdate::Snapshot {
                    messages: lines(chat, theme),
                    status: status(chat),
                }
                .send_signal_to_dart();
            }
            ChatChange::RoomChanged | ChatChange::ConnectionChanged | ChatChange::NoticeChanged => {
                send_batch(batch);
                ChatUpdate::Status {
                    status: status(chat),
                }
                .send_signal_to_dart();
            }
        }
    }
}

/// When the waiting batch is due to go out; `None` while it is empty.
pub fn flush_at(panel: &ChatPanel) -> Option<Instant> {
    panel.batch.due
}

/// The batch window ended: sends what waits in it.
pub fn flush(panel: &mut ChatPanel) {
    send_batch(&mut panel.batch);
}

/// Resolves at `deadline`, or never without one: a `select!` arm that is
/// idle while no batch waits.
pub async fn until(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,
        None => std::future::pending().await,
    }
}

fn send_batch(batch: &mut Batch) {
    let Batch {
        messages, evicted, ..
    } = std::mem::take(batch);
    if !messages.is_empty() {
        ChatUpdate::Appended { messages }.send_signal_to_dart();
    }
    if evicted > 0 {
        let count = u32::try_from(evicted).unwrap_or(u32::MAX);
        ChatUpdate::Evicted { count }.send_signal_to_dart();
    }
}

/// The whole panel; whatever waited in the batch is part of it.
fn send_snapshot(panel: &mut ChatPanel) {
    panel.batch = Batch::default();
    match &panel.live {
        None => ChatUpdate::Off,
        Some(live) => ChatUpdate::Snapshot {
            messages: lines(&live.chat, panel.theme),
            status: status(&live.chat),
        },
    }
    .send_signal_to_dart();
}

fn lines(chat: &ChatController, theme: Theme) -> Vec<ChatLine> {
    chat.messages().map(|m| line(chat, m, theme)).collect()
}

fn line(chat: &ChatController, message: &ChatMessage, theme: Theme) -> ChatLine {
    let sender = &message.sender;
    let color = name_color(sender.color, &sender.user_id, theme);
    ChatLine {
        id: message.id.clone(),
        name: sender.display_name.clone(),
        color: 0xFF00_0000
            | u32::from(color.r) << 16
            | u32::from(color.g) << 8
            | u32::from(color.b),
        action: message.action,
        deleted: message.deleted,
        fragments: chat
            .fragments(message)
            .into_iter()
            .map(ChatFragment::from)
            .collect(),
    }
}

fn status(chat: &ChatController) -> ChatStatus {
    ChatStatus {
        banner: match chat.connection() {
            Connection::Connected => None,
            Connection::Disconnected => Some("Connecting to chat…".to_owned()),
        },
        modes: modes(chat.room()),
        notice: chat.notice().map(str::to_owned),
        emote_hint: emote_hint(chat.failed_providers()),
    }
}

fn modes(room: RoomState) -> Vec<String> {
    let slow = (room.slow != Seconds::ZERO).then(|| format!("Slow mode ({}s)", room.slow.get()));
    [
        room.emote_only.then(|| "Emote-only".to_owned()),
        room.subs_only.then(|| "Subscribers-only".to_owned()),
        room.r9k.then(|| "Unique messages".to_owned()),
        slow,
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// `Fewer emotes: BTTV, FFZ did not load`.
fn emote_hint(failed: &[EmoteProvider]) -> Option<String> {
    let names: Vec<&str> = failed.iter().map(|p| provider_name(*p)).collect();
    (!names.is_empty()).then(|| format!("Fewer emotes: {} did not load", names.join(", ")))
}

fn provider_name(provider: EmoteProvider) -> &'static str {
    match provider {
        EmoteProvider::SevenTv => "7TV",
        EmoteProvider::Bttv => "BTTV",
        EmoteProvider::Ffz => "FFZ",
    }
}

impl From<Fragment> for ChatFragment {
    fn from(fragment: Fragment) -> Self {
        match fragment {
            Fragment::Text(text) => Self::Text { text },
            Fragment::Emote(emote) => Self::Emote {
                emote: EmoteImage::from(emote),
            },
            Fragment::Overlay { base, overlays } => Self::Overlay {
                base: EmoteImage::from(base),
                overlays: overlays.into_iter().map(EmoteImage::from).collect(),
            },
            Fragment::Link(text) => Self::Link {
                url: link_url(&text),
                text,
            },
        }
    }
}

impl From<Emote> for EmoteImage {
    fn from(emote: Emote) -> Self {
        Self {
            name: emote.name,
            url: emote.url,
        }
    }
}

/// Core links start with `http://`, `https://` or `www.`; the last opens
/// over https.
fn link_url(text: &str) -> String {
    match text.starts_with("www.") {
        true => format!("https://{text}"),
        false => text.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{emote_hint, link_url, modes};
    use jitter_core::models::{EmoteProvider, RoomState, Seconds};

    #[test]
    fn www_links_open_over_https() {
        assert_eq!(link_url("www.twitch.tv"), "https://www.twitch.tv");
        assert_eq!(link_url("http://a.b"), "http://a.b");
    }

    #[test]
    fn modes_list_only_those_on() {
        assert!(modes(RoomState::default()).is_empty());
        let room = RoomState {
            subs_only: true,
            slow: Seconds::new(30),
            ..RoomState::default()
        };
        assert_eq!(modes(room), ["Subscribers-only", "Slow mode (30s)"]);
    }

    #[test]
    fn the_hint_names_the_failed_providers() {
        assert_eq!(emote_hint(&[]), None);
        assert_eq!(
            emote_hint(&[EmoteProvider::Bttv, EmoteProvider::Ffz]).as_deref(),
            Some("Fewer emotes: BTTV, FFZ did not load")
        );
    }
}
