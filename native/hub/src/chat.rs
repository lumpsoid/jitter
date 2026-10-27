//! The chat panel: a live channel's chat connection, or a VOD's chat
//! replay, with the emote fetches for the channel being played; the chat
//! controllers ↔ chat signals; and the batching of appended messages on
//! their way to Dart.
//!
//! Each `Play` starts a session: a fresh controller and the tasks feeding
//! it, whose results come back as [`Fetched::Chat`] tagged with the
//! session. Ending the session aborts its tasks, and a result already on
//! its way from them is dropped by its tag, so only one connection or
//! replay ever feeds the panel.
//!
//! A replay is core's [`ChatReplay`]: the player's position goes in
//! ([`position`]), and the pages it asks for are fetched and handed back.
//!
//! Whether a session runs at all is core's [`ChatPanelController`]: the
//! user can hide the panel, which ends the session, and show it again,
//! which starts a new one.
//!
//! The user's settings reach the panel through [`settings_changed`]: a
//! player page opening with auto-load off starts hidden, a session asks
//! only for the emote providers the settings name, and lines show still
//! emotes with animation off.

use std::time::Duration;

use futures_util::StreamExt;
use jitter_core::chat::{ChatController, Connection};
use jitter_core::chat_panel::{ChatPanelController, ChatPanelState, SessionChange};
use jitter_core::chat_replay::{ChatReplay, ReplayRequest, ReplayStep};
use jitter_core::models::{
    ChannelId, ChannelLogin, ChatChange, ChatEvent, ChatMessage, Emote, EmoteProvider, EmoteScope,
    Fragment, Page, ReplayComment, RoomState, Seconds, Settings, VideoId,
};
use jitter_core::name_color::{Theme, name_color};
use jitter_core::playback::PlaybackTarget;
use jitter_twitch::Twitch;
use jitter_twitch::emotes::ProviderEmotes;
use rinf::{RustSignal, debug_print};
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;
use tokio::time::Instant;

use crate::format::twitch_error;
use crate::run::Fetched;
use crate::signals::{ChatFragment, ChatLine, ChatStatus, ChatUpdate, EmoteImage};

/// Appended messages are gathered this long and sent as one `Appended`:
/// transport batching, so a busy channel (~8 messages a second) costs
/// Dart a rebuild per window instead of per message.
const BATCH_WINDOW: Duration = Duration::from_millis(100);

/// The hub's side of the chat panel.
pub struct ChatPanel {
    /// Whether a session should run: something plays, chat not hidden.
    view: ChatPanelController,
    session: Option<Session>,
    /// Sessions started so far; the latest one's tag.
    sessions: u64,
    theme: Theme,
    /// The user's settings, as the hub last passed them.
    settings: Settings,
    batch: Batch,
}

/// How lines render: name colours for the background, and whether emotes
/// move.
#[derive(Clone, Copy)]
struct Look {
    theme: Theme,
    animated_emotes: bool,
}

/// The chat of what is being played.
struct Session {
    tag: u64,
    chat: SessionChat,
    _tasks: Tasks,
}

/// A live channel's chat, or a VOD's replay.
enum SessionChat {
    Live(ChatController),
    Replay(ChatReplay),
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

/// One provider's emotes, or why they did not load.
type ProviderResult = (EmoteProvider, Result<Vec<Emote>, String>);

/// A chat task's output, back in the loop as [`Fetched::Chat`].
pub struct ChatFetched {
    session: u64,
    payload: Payload,
}

enum Payload {
    Event(ChatEvent),
    Emotes {
        scope: EmoteScope,
        results: Vec<ProviderResult>,
    },
    /// A replay page, the error already in words for the panel.
    ReplayPage {
        request: ReplayRequest,
        result: Result<Page<ReplayComment>, String>,
    },
}

impl ChatPanel {
    /// Empty, under the user's `settings` as loaded at boot.
    pub fn new(settings: Settings) -> Self {
        Self {
            view: ChatPanelController::new(),
            session: None,
            sessions: 0,
            theme: Theme::Dark,
            settings,
            batch: Batch::default(),
        }
    }

    fn look(&self) -> Look {
        Look {
            theme: self.theme,
            animated_emotes: self.settings.animated_emotes,
        }
    }
}

impl SessionChat {
    /// The messages on screen and how to render them.
    fn chat(&self) -> &ChatController {
        match self {
            Self::Live(chat) => chat,
            Self::Replay(replay) => replay.chat(),
        }
    }

    fn set_emotes(&mut self, scope: EmoteScope, provider: EmoteProvider, emotes: Vec<Emote>) {
        match self {
            Self::Live(chat) => chat.set_emotes(scope, provider, emotes),
            Self::Replay(replay) => replay.set_emotes(scope, provider, emotes),
        };
    }

    fn provider_failed(&mut self, provider: EmoteProvider) {
        match self {
            Self::Live(chat) => chat.provider_failed(provider),
            Self::Replay(replay) => replay.provider_failed(provider),
        }
    }

    fn status(&self) -> ChatStatus {
        match self {
            Self::Live(chat) => status(chat),
            Self::Replay(replay) => replay_status(replay),
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

/// The user changed a setting: lines render with the new settings at
/// once; auto-load and the emote providers apply from the next session.
pub fn settings_changed(panel: &mut ChatPanel, settings: Settings) {
    panel.settings = settings;
    send_snapshot(panel);
}

/// A player page opened on `target` (`None`: core rejected it). A shown
/// live channel or VOD gets a new session; anything else ends the current
/// one. Core starts the panel hidden when the settings do not auto-load.
pub fn play(
    panel: &mut ChatPanel,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
    target: Option<&PlaybackTarget>,
) {
    let change = panel.view.play(target, panel.settings.auto_load_chat);
    apply(panel, twitch, fetch, change);
}

/// The player stopped: ends the session.
pub fn stop(panel: &mut ChatPanel, twitch: &Twitch, fetch: &UnboundedSender<Fetched>) {
    let change = panel.view.stop();
    apply(panel, twitch, fetch, change);
}

/// `HideChat`: ends the session; the panel shows as hidden.
pub fn hide(panel: &mut ChatPanel, twitch: &Twitch, fetch: &UnboundedSender<Fetched>) {
    let change = panel.view.hide();
    apply(panel, twitch, fetch, change);
}

/// `ShowChat`: joins the playing channel's chat, or replays the playing
/// VOD's, again.
pub fn show(panel: &mut ChatPanel, twitch: &Twitch, fetch: &UnboundedSender<Fetched>) {
    let change = panel.view.show();
    apply(panel, twitch, fetch, change);
}

/// The player showing `target` is at `position` of `duration`: a running
/// replay shows what was said up to there and fetches what it asks for.
/// Live chat has nothing to do with it.
pub fn position(
    panel: &mut ChatPanel,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
    target: &PlaybackTarget,
    position: Seconds,
    duration: Seconds,
) {
    let look = panel.look();
    let ChatPanel { session, batch, .. } = panel;
    let Some(Session { tag, chat, .. }) = session.as_mut() else {
        return;
    };
    let SessionChat::Replay(replay) = chat else {
        return;
    };
    let step = replay.position(target, position, duration);
    carry_out(chat, *tag, look, batch, step, twitch, fetch);
}

/// Carries out core's session change, then sends the panel.
fn apply(
    panel: &mut ChatPanel,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
    change: SessionChange,
) {
    match change {
        SessionChange::Join(login) => start_live(panel, twitch, fetch, &login),
        SessionChange::Replay(video) => start_replay(panel, twitch, fetch, video),
        // Dropping the session aborts its connection and fetches.
        SessionChange::Leave => panel.session = None,
        SessionChange::Keep => {}
    }
    send_snapshot(panel);
}

fn start_live(
    panel: &mut ChatPanel,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
    login: &ChannelLogin,
) {
    panel.sessions += 1;
    let tag = panel.sessions;
    let providers = panel.settings.emote_providers();
    let tasks = Tasks(vec![
        spawn_connection(tag, login.clone(), fetch.clone()),
        spawn_global_emotes(tag, providers, twitch.clone(), fetch.clone()),
        spawn_channel_emotes(tag, login.clone(), providers, twitch.clone(), fetch.clone()),
    ]);
    // Replacing the session drops (and so aborts) the previous tasks.
    panel.session = Some(Session {
        tag,
        chat: SessionChat::Live(ChatController::new()),
        _tasks: tasks,
    });
}

/// The replay fetches nothing until the player reports a position; its
/// emotes load meanwhile.
fn start_replay(
    panel: &mut ChatPanel,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
    video: VideoId,
) {
    panel.sessions += 1;
    let tag = panel.sessions;
    let providers = panel.settings.emote_providers();
    let tasks = Tasks(vec![
        spawn_global_emotes(tag, providers, twitch.clone(), fetch.clone()),
        spawn_owner_emotes(tag, video.clone(), providers, twitch.clone(), fetch.clone()),
    ]);
    panel.session = Some(Session {
        tag,
        chat: SessionChat::Replay(ChatReplay::new(video)),
        _tasks: tasks,
    });
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

/// Fetches one replay page. Not held with the session's tasks: it ends by
/// itself, and its result is dropped by tag once the session has.
fn spawn_page(
    session: u64,
    request: ReplayRequest,
    twitch: Twitch,
    fetch: UnboundedSender<Fetched>,
) {
    tokio::spawn(async move {
        let result = twitch
            .video_comments(&request.video, &request.from)
            .await
            .inspect_err(|e| debug_print!("loading the chat replay: {e}"))
            .map_err(|e| format!("Couldn't load the chat. {}", twitch_error(&e)));
        let fetched = ChatFetched {
            session,
            payload: Payload::ReplayPage { request, result },
        };
        let _ = fetch.send(Fetched::Chat(fetched));
    });
}

/// The global emotes of `providers`, which core's settings chose; none
/// asks no provider.
fn spawn_global_emotes(
    session: u64,
    providers: &'static [EmoteProvider],
    twitch: Twitch,
    fetch: UnboundedSender<Fetched>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let results = with_messages(twitch.global_emotes(providers).await);
        send_emotes(&fetch, session, EmoteScope::Global, results);
    })
}

/// The providers key channel emotes by channel id, so the channel is
/// looked up first; if that fails, every provider counts as failed.
fn spawn_channel_emotes(
    session: u64,
    login: ChannelLogin,
    providers: &'static [EmoteProvider],
    twitch: Twitch,
    fetch: UnboundedSender<Fetched>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let results = match twitch.channel(&login).await {
            Ok(channel) => with_messages(twitch.channel_emotes(&channel.id, providers).await),
            Err(e) => all_failed(providers, &format!("looking up {login}: {e}")),
        };
        send_emotes(&fetch, session, EmoteScope::Channel, results);
    })
}

/// A VOD's channel emotes: those of the channel that streamed it, looked
/// up first by the VOD's id.
fn spawn_owner_emotes(
    session: u64,
    video: VideoId,
    providers: &'static [EmoteProvider],
    twitch: Twitch,
    fetch: UnboundedSender<Fetched>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let owner: Result<ChannelId, String> = twitch
            .playback_details(&PlaybackTarget::Vod(video.clone()))
            .await
            .map_err(|e| e.to_string())
            .and_then(|details| details.channel.ok_or_else(|| "no channel".to_owned()))
            .map(|channel| channel.id);
        let results = match owner {
            Ok(id) => with_messages(twitch.channel_emotes(&id, providers).await),
            Err(e) => all_failed(
                providers,
                &format!("looking up the channel of VOD {video}: {e}"),
            ),
        };
        send_emotes(&fetch, session, EmoteScope::Channel, results);
    })
}

fn all_failed(providers: &[EmoteProvider], message: &str) -> Vec<ProviderResult> {
    providers
        .iter()
        .map(|&provider| (provider, Err(message.to_owned())))
        .collect()
}

fn with_messages(results: Vec<ProviderEmotes>) -> Vec<ProviderResult> {
    results
        .into_iter()
        .map(|(provider, result)| (provider, result.map_err(|e| e.to_string())))
        .collect()
}

fn send_emotes(
    fetch: &UnboundedSender<Fetched>,
    session: u64,
    scope: EmoteScope,
    results: Vec<ProviderResult>,
) {
    let fetched = ChatFetched {
        session,
        payload: Payload::Emotes { scope, results },
    };
    let _ = fetch.send(Fetched::Chat(fetched));
}

/// Hands a chat task's output to the controller and passes on what
/// changed. Output of an ended session is dropped.
pub fn on_fetched(
    panel: &mut ChatPanel,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
    fetched: ChatFetched,
) {
    let look = panel.look();
    let ChatPanel { session, batch, .. } = panel;
    let Some(Session { tag, chat, .. }) = session.as_mut().filter(|s| s.tag == fetched.session)
    else {
        return;
    };
    match (fetched.payload, &mut *chat) {
        (Payload::Event(event), SessionChat::Live(live)) => {
            let changes = live.apply(event);
            pass_on(chat, look, batch, changes);
        }
        (Payload::ReplayPage { request, result }, SessionChat::Replay(replay)) => {
            let step = replay.apply(request, result);
            carry_out(chat, *tag, look, batch, step, twitch, fetch);
        }
        // A session's tasks only send what its kind of chat takes.
        (Payload::Event(_) | Payload::ReplayPage { .. }, _) => {}
        (Payload::Emotes { scope, results }, _) => {
            for (provider, result) in results {
                match result {
                    Ok(emotes) => chat.set_emotes(scope, provider, emotes),
                    Err(e) => {
                        debug_print!("{provider:?} {scope:?} emotes unavailable: {e}");
                        chat.provider_failed(provider);
                    }
                }
            }
            // `set_emotes` always asks for a rerender and a failure changes
            // the hint, so one snapshot covers every result.
            send_snapshot(panel);
        }
    }
}

/// Passes on what a replay step changed, then fetches the page it asks
/// for.
fn carry_out(
    chat: &SessionChat,
    tag: u64,
    look: Look,
    batch: &mut Batch,
    step: ReplayStep,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
) {
    pass_on(chat, look, batch, step.changes);
    if let Some(request) = step.fetch {
        spawn_page(tag, request, twitch.clone(), fetch.clone());
    }
}

/// Sends each change as its diff, in order. Appends and evictions wait in
/// the batch; anything else sends the batch first, so Dart sees every
/// change in the order core made it.
fn pass_on(session: &SessionChat, look: Look, batch: &mut Batch, changes: Vec<ChatChange>) {
    let chat = session.chat();
    for change in changes {
        match change {
            ChatChange::Appended { message } => {
                batch.messages.push(line(chat, &message, look));
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
                    messages: lines(chat, look),
                    status: session.status(),
                }
                .send_signal_to_dart();
            }
            ChatChange::RoomChanged | ChatChange::ConnectionChanged | ChatChange::NoticeChanged => {
                send_batch(batch);
                ChatUpdate::Status {
                    status: session.status(),
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
    match (panel.view.state(), &panel.session) {
        (ChatPanelState::Shown, Some(session)) => ChatUpdate::Snapshot {
            messages: lines(session.chat.chat(), panel.look()),
            status: session.chat.status(),
        },
        (ChatPanelState::Hidden, _) => ChatUpdate::Hidden,
        (ChatPanelState::Off, _) | (ChatPanelState::Shown, None) => ChatUpdate::Off,
    }
    .send_signal_to_dart();
}

fn lines(chat: &ChatController, look: Look) -> Vec<ChatLine> {
    chat.messages().map(|m| line(chat, m, look)).collect()
}

fn line(chat: &ChatController, message: &ChatMessage, look: Look) -> ChatLine {
    let sender = &message.sender;
    let color = name_color(sender.color, &sender.user_id, look.theme);
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
            .map(|fragment| chat_fragment(fragment, look.animated_emotes))
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

/// A replay has no room or connection: the banner shows while its first
/// page loads, and the notice says why it stopped.
fn replay_status(replay: &ChatReplay) -> ChatStatus {
    ChatStatus {
        banner: replay.waiting().then(|| "Loading chat…".to_owned()),
        modes: Vec::new(),
        notice: replay.failure().map(str::to_owned),
        emote_hint: emote_hint(replay.chat().failed_providers()),
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

/// One fragment as the panel draws it; core's `Emote::image_url` picks
/// each emote's image for `animated`.
fn chat_fragment(fragment: Fragment, animated: bool) -> ChatFragment {
    match fragment {
        Fragment::Text(text) => ChatFragment::Text { text },
        Fragment::Emote(emote) => ChatFragment::Emote {
            emote: emote_image(&emote, animated),
        },
        Fragment::Overlay { base, overlays } => ChatFragment::Overlay {
            base: emote_image(&base, animated),
            overlays: overlays
                .iter()
                .map(|overlay| emote_image(overlay, animated))
                .collect(),
        },
        Fragment::Link(text) => ChatFragment::Link {
            url: link_url(&text),
            text,
        },
    }
}

fn emote_image(emote: &Emote, animated: bool) -> EmoteImage {
    EmoteImage {
        name: emote.name.clone(),
        url: emote.image_url(animated).to_owned(),
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
