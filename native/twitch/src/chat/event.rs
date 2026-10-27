//! Maps parsed IRC lines to core chat events. Pure.

use jitter_core::models::{
    Badge, ChannelId, ChatEvent, ChatMessage, Chatter, EmoteSpan, Rgb, RoomStateUpdate, Seconds,
    Timestamp,
};

use super::irc::IrcMessage;
use crate::emotes::twitch_emote;

/// How a `/me` message is wrapped: `\u{1}ACTION <text>\u{1}`.
const ACTION_PREFIX: &str = "\u{1}ACTION ";
const ACTION_SUFFIX: char = '\u{1}';

/// The event a line stands for, or `None` for lines that are not chat
/// events (PING, RECONNECT, numerics, JOIN, ...) and for lines missing data
/// their event needs. Connection control is the caller's business.
///
/// A USERNOTICE with user text (e.g. an announcement) is a `Message`; one
/// without (a bare sub) is a `Notice` carrying `system-msg`. `system-msg` of
/// a USERNOTICE that also has text is dropped.
pub fn to_event(msg: &IrcMessage) -> Option<ChatEvent> {
    match msg.command.as_str() {
        "PRIVMSG" => message(msg).map(ChatEvent::Message),
        "USERNOTICE" => match msg.trailing.as_deref() {
            Some(_) => message(msg).map(ChatEvent::Message),
            None => non_empty(msg.tag("system-msg")?).map(|s| ChatEvent::Notice(s.to_owned())),
        },
        "CLEARMSG" => Some(ChatEvent::ClearMessage {
            id: non_empty(msg.tag("target-msg-id")?)?.to_owned(),
        }),
        // A target means a timeout or ban of that user; none means the chat
        // was cleared.
        "CLEARCHAT" => match msg.tag("target-user-id") {
            Some(id) => Some(ChatEvent::ClearUser {
                user_id: ChannelId::parse(id).ok()?,
            }),
            None => Some(ChatEvent::ClearAll),
        },
        "NOTICE" => Some(ChatEvent::Notice(msg.trailing.clone()?)),
        "ROOMSTATE" => Some(ChatEvent::RoomState(room_state(msg))),
        _ => None,
    }
}

fn non_empty(s: &str) -> Option<&str> {
    (!s.is_empty()).then_some(s)
}

fn flag(msg: &IrcMessage, key: &str) -> Option<bool> {
    match msg.tag(key)? {
        "1" => Some(true),
        "0" => Some(false),
        _ => None,
    }
}

/// Only the fields the line carries; ROOMSTATE after join has all, later
/// ones have one. `followers-only` is not modelled.
fn room_state(msg: &IrcMessage) -> RoomStateUpdate {
    RoomStateUpdate {
        emote_only: flag(msg, "emote-only"),
        subs_only: flag(msg, "subs-only"),
        r9k: flag(msg, "r9k"),
        slow: msg
            .tag("slow")
            .and_then(|s| s.parse::<u32>().ok())
            .map(Seconds::new),
    }
}

fn message(msg: &IrcMessage) -> Option<ChatMessage> {
    let raw = msg.trailing.as_deref()?;
    let (text, action) = match raw
        .strip_prefix(ACTION_PREFIX)
        .map(|t| t.strip_suffix(ACTION_SUFFIX).unwrap_or(t))
    {
        Some(text) => (text, true),
        None => (raw, false),
    };

    let display_name = msg
        .tag("display-name")
        .and_then(non_empty)
        .or_else(|| msg.nick())?;
    let sender = Chatter {
        user_id: ChannelId::parse(msg.tag("user-id")?).ok()?,
        display_name: display_name.to_owned(),
        color: msg.tag("color").and_then(Rgb::parse_hex),
        badges: msg.tag("badges").map(badges).unwrap_or_default(),
    };
    let millis: i64 = msg.tag("tmi-sent-ts")?.parse().ok()?;

    Some(ChatMessage {
        id: non_empty(msg.tag("id")?)?.to_owned(),
        sender,
        text: text.to_owned(),
        action,
        emotes: emote_spans(msg.tag("emotes").unwrap_or(""), text),
        sent_at: Timestamp::from_unix(millis / 1000),
        deleted: false,
    })
}

/// `set/version,set/version`; entries without a `/` are dropped.
fn badges(tag: &str) -> Vec<Badge> {
    tag.split(',')
        .filter_map(|entry| {
            let (set_id, version) = entry.split_once('/')?;
            Some(Badge {
                set_id: non_empty(set_id)?.to_owned(),
                version: version.to_owned(),
            })
        })
        .collect()
}

/// `id:begin-end,begin-end/id:begin-end`, indices in code points, end
/// inclusive, into `text` (without any ACTION wrapper). A range that does
/// not fit the text is dropped. Sorted by position.
fn emote_spans(tag: &str, text: &str) -> Vec<EmoteSpan> {
    let chars: Vec<char> = text.chars().collect();
    let mut spans = Vec::new();
    for entry in tag.split('/').filter(|e| !e.is_empty()) {
        let Some((id, ranges)) = entry.split_once(':') else {
            continue;
        };
        for range in ranges.split(',') {
            let Some((start, end)) = range.split_once('-') else {
                continue;
            };
            let (Ok(start), Ok(end)) = (start.parse::<usize>(), end.parse::<usize>()) else {
                continue;
            };
            let Some(name) = chars.get(start..=end) else {
                continue;
            };
            spans.push(EmoteSpan {
                start,
                end,
                emote: twitch_emote(id.to_owned(), name.iter().collect()),
            });
        }
    }
    spans.sort_by_key(|s| s.start);
    spans
}
