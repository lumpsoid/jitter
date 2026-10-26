//! The anonymous chat connection: one WebSocket to Twitch IRC that logs in,
//! joins a channel, keeps itself alive and reconnects by itself.
//!
//! The decisions are pure and tested with plain values: `backoff` and
//! `Retry` (how long to wait before the next attempt), `next_action` (what
//! an incoming line asks of the connection) and `Keepalive` (when to send the
//! client PING and when a missing PONG means the link is dead). The async loop below only
//! reads, writes and sleeps on their answers.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use futures_util::stream::{self, Stream};
use futures_util::{SinkExt, StreamExt};
use jitter_core::models::{ChannelLogin, ChatEvent};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use super::{parse, split_frame, to_event};

/// Twitch IRC over WebSocket.
pub const IRC_URL: &str = "wss://irc-ws.chat.twitch.tv";

/// Tags give message metadata (emotes, badges, ids); commands give
/// CLEARCHAT, CLEARMSG, ROOMSTATE, USERNOTICE, NOTICE and RECONNECT.
const CAP_REQ: &str = "CAP REQ :twitch.tv/tags twitch.tv/commands";

/// Anonymous login: any `justinfan<1000..9999>` nick, no password.
const ANONYMOUS_NICKS: std::ops::Range<u32> = 1000..10000;

/// The client sends `PING` this long after connecting or after the last
/// PONG.
pub const PING_INTERVAL: Duration = Duration::from_secs(270);

/// A PING unanswered for this long means the link is dead.
pub const PONG_TIMEOUT: Duration = Duration::from_secs(10);

/// The first retry waits this long; each further failed attempt doubles it.
pub const BACKOFF_START: Duration = Duration::from_secs(1);

/// Retries never wait longer than this.
pub const BACKOFF_CAP: Duration = Duration::from_secs(60);

/// How long to wait before reconnect attempt `attempt` (0-based, counted
/// since the last successful JOIN): 1 s, 2 s, 4 s … capped at 60 s.
pub fn backoff(attempt: u32) -> Duration {
    BACKOFF_START
        .checked_mul(2u32.saturating_pow(attempt))
        .map_or(BACKOFF_CAP, |delay| delay.min(BACKOFF_CAP))
}

/// Failed attempts since the last successful JOIN, turned into the wait
/// before the next one. A JOIN resets it, so a link that drops after an hour
/// of chat retries after 1 s, not after the cap.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Retry {
    attempt: u32,
}

impl Retry {
    /// The link failed: how long to wait before trying again.
    pub fn on_failure(&mut self) -> Duration {
        let delay = backoff(self.attempt);
        self.attempt = self.attempt.saturating_add(1);
        delay
    }

    pub fn on_joined(&mut self) {
        self.attempt = 0;
    }
}

/// What one incoming line asks of the connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Send this line back (the answer to a server PING).
    Reply(String),
    /// The server is going away: drop the socket and connect again now.
    Reconnect,
    /// The answer to our client PING.
    Pong,
    /// The channel is joined: the connection is up.
    Joined,
    /// A chat event for the head.
    Event(Box<ChatEvent>),
    /// Nothing to do (CAP ACK, numerics, unmapped or malformed lines).
    Ignore,
}

/// Decides what an incoming IRC line asks for. Connection-level commands
/// are matched before the line is mapped to a chat event, because
/// `to_event` does not handle them.
pub fn next_action(line: &str) -> Action {
    let Some(msg) = parse(line) else {
        return Action::Ignore;
    };
    match msg.command.as_str() {
        // `PING :tmi.twitch.tv` → `PONG :tmi.twitch.tv`.
        "PING" => Action::Reply(match &msg.trailing {
            Some(token) => format!("PONG :{token}"),
            None => "PONG".to_owned(),
        }),
        "PONG" => Action::Pong,
        "RECONNECT" => Action::Reconnect,
        // Without the membership capability Twitch echoes only our own JOIN.
        "JOIN" => Action::Joined,
        _ => to_event(&msg).map_or(Action::Ignore, |event| Action::Event(Box::new(event))),
    }
}

/// The client keepalive: PING `PING_INTERVAL` after connecting or after the
/// last PONG, and give up `PONG_TIMEOUT` after a PING with no answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keepalive {
    Idle { ping_at: Instant },
    AwaitingPong { deadline: Instant },
}

/// What the keepalive timer asks for when it fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeepaliveStep {
    SendPing,
    TimedOut,
}

impl Keepalive {
    pub fn new(now: Instant) -> Self {
        Self::Idle {
            ping_at: now + PING_INTERVAL,
        }
    }

    /// When the timer next needs attention.
    pub fn wake_at(&self) -> Instant {
        match *self {
            Self::Idle { ping_at } => ping_at,
            Self::AwaitingPong { deadline } => deadline,
        }
    }

    /// Called when the timer fires; `None` if it fired early.
    pub fn on_timer(&mut self, now: Instant) -> Option<KeepaliveStep> {
        if now < self.wake_at() {
            return None;
        }
        match self {
            Self::Idle { .. } => {
                *self = Self::AwaitingPong {
                    deadline: now + PONG_TIMEOUT,
                };
                Some(KeepaliveStep::SendPing)
            }
            Self::AwaitingPong { .. } => Some(KeepaliveStep::TimedOut),
        }
    }

    pub fn on_pong(&mut self, now: Instant) {
        *self = Self::new(now);
    }
}

/// Chat events for `login`, forever: the stream reconnects by itself and
/// reports each change of connection state as `Connected`/`Disconnected`.
pub fn connect(login: ChannelLogin) -> impl Stream<Item = ChatEvent> {
    stream::unfold(Connection::new(login), |mut connection| async move {
        let event = connection.next_event().await;
        Some((event, connection))
    })
}

type Socket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

struct Live {
    socket: Socket,
    keepalive: Keepalive,
}

/// What woke the read loop.
enum Wake {
    Frame(Option<Result<Message, tokio_tungstenite::tungstenite::Error>>),
    Timer,
}

struct Connection {
    login: ChannelLogin,
    live: Option<Live>,
    retry: Retry,
    /// Waited out before the next connect; `None` connects at once.
    delay: Option<Duration>,
    /// Whether `Connected` was the last state reported.
    joined: bool,
    pending: VecDeque<ChatEvent>,
}

impl Connection {
    fn new(login: ChannelLogin) -> Self {
        Self {
            login,
            live: None,
            retry: Retry::default(),
            delay: None,
            joined: false,
            pending: VecDeque::new(),
        }
    }

    async fn next_event(&mut self) -> ChatEvent {
        loop {
            if let Some(event) = self.pending.pop_front() {
                return event;
            }
            match self.live.as_mut() {
                None => self.open().await,
                Some(live) => {
                    let wake_at = tokio::time::Instant::from_std(live.keepalive.wake_at());
                    let wake = tokio::select! {
                        frame = live.socket.next() => Wake::Frame(frame),
                        () = tokio::time::sleep_until(wake_at) => Wake::Timer,
                    };
                    self.on_wake(wake).await;
                }
            }
        }
    }

    async fn open(&mut self) {
        if let Some(delay) = self.delay.take() {
            log::debug!("chat: reconnecting in {} s", delay.as_secs());
            tokio::time::sleep(delay).await;
        }
        log::debug!("chat: connecting to {IRC_URL}");
        let mut socket = match connect_async(IRC_URL).await {
            Ok((socket, _)) => socket,
            Err(e) => return self.dropped(&format!("connect failed: {e}")),
        };
        let nick = rand::random_range(ANONYMOUS_NICKS);
        for line in [
            CAP_REQ.to_owned(),
            format!("NICK justinfan{nick}"),
            format!("JOIN #{}", self.login),
        ] {
            if let Err(e) = socket.send(Message::text(line)).await {
                return self.dropped(&format!("login failed: {e}"));
            }
        }
        self.live = Some(Live {
            socket,
            keepalive: Keepalive::new(Instant::now()),
        });
    }

    async fn on_wake(&mut self, wake: Wake) {
        match wake {
            Wake::Frame(Some(Ok(Message::Text(frame)))) => {
                for line in split_frame(frame.as_str()) {
                    self.on_action(next_action(line)).await;
                    if self.live.is_none() {
                        break;
                    }
                }
            }
            Wake::Frame(Some(Ok(Message::Close(frame)))) => {
                self.dropped(&format!("server closed the socket: {frame:?}"));
            }
            // WebSocket pings are answered by tungstenite itself.
            Wake::Frame(Some(Ok(_))) => {}
            Wake::Frame(Some(Err(e))) => self.dropped(&format!("read failed: {e}")),
            Wake::Frame(None) => self.dropped("socket ended"),
            Wake::Timer => self.on_timer().await,
        }
    }

    async fn on_action(&mut self, action: Action) {
        match action {
            Action::Reply(line) => {
                log::debug!("chat: server PING, answering {line:?}");
                self.send(line).await;
            }
            Action::Reconnect => {
                log::debug!("chat: server asked to reconnect");
                if let Some(mut live) = self.live.take()
                    && let Err(e) = live.socket.close(None).await
                {
                    log::debug!("chat: closing before reconnect failed: {e}");
                }
                self.report_down();
                self.delay = None;
            }
            Action::Pong => {
                log::debug!("chat: PONG received");
                if let Some(live) = self.live.as_mut() {
                    live.keepalive.on_pong(Instant::now());
                }
            }
            Action::Joined => {
                log::debug!("chat: joined #{}", self.login);
                self.retry.on_joined();
                if !self.joined {
                    self.joined = true;
                    self.pending.push_back(ChatEvent::Connected);
                }
            }
            Action::Event(event) => self.pending.push_back(*event),
            Action::Ignore => {}
        }
    }

    async fn on_timer(&mut self) {
        let Some(live) = self.live.as_mut() else {
            return;
        };
        match live.keepalive.on_timer(Instant::now()) {
            Some(KeepaliveStep::SendPing) => {
                log::debug!("chat: sending client PING");
                self.send("PING".to_owned()).await;
            }
            Some(KeepaliveStep::TimedOut) => {
                self.dropped(&format!("no PONG within {} s", PONG_TIMEOUT.as_secs()));
            }
            None => {}
        }
    }

    async fn send(&mut self, line: String) {
        let Some(live) = self.live.as_mut() else {
            return;
        };
        if let Err(e) = live.socket.send(Message::text(line)).await {
            self.dropped(&format!("write failed: {e}"));
        }
    }

    /// The link failed: forget the socket and wait `backoff` before the
    /// next attempt.
    fn dropped(&mut self, reason: &str) {
        log::debug!("chat: connection dropped: {reason}");
        self.live = None;
        self.report_down();
        self.delay = Some(self.retry.on_failure());
    }

    fn report_down(&mut self) {
        if self.joined {
            self.joined = false;
            self.pending.push_back(ChatEvent::Disconnected);
        }
    }
}
