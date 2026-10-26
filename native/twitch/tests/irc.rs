//! IRC parser and event mapping. Lines under a `# <kind>` header of
//! `irc_lines.txt` are recorded from Twitch. Hand-written (Twitch did not
//! produce them in the recording session): partial ROOMSTATE, CLEARCHAT
//! without a target, permanent-ban CLEARCHAT, NOTICE, RECONNECT, tag values
//! containing `=`, `\:` and `\\`, and the malformed lines.

use jitter_core::models::{ChatEvent, Rgb, RoomStateUpdate, Seconds};
use jitter_twitch::chat::{IrcMessage, parse, split_frame, to_event};

const RECORDED: &str = include_str!("fixtures/irc_lines.txt");

/// The recorded line under the `# <kind>` header.
fn recorded(kind: &str) -> &'static str {
    let header = format!("# {kind}");
    let mut lines = RECORDED.lines();
    while let Some(line) = lines.next() {
        if line == header
            && let Some(found) = lines.next()
        {
            return found;
        }
    }
    unreachable!("no recorded line `{kind}`")
}

fn parsed(line: &str) -> IrcMessage {
    match parse(line) {
        Some(m) => m,
        None => unreachable!("did not parse: {line}"),
    }
}

fn event(line: &str) -> ChatEvent {
    match to_event(&parsed(line)) {
        Some(e) => e,
        None => unreachable!("no event for: {line}"),
    }
}

fn message(line: &str) -> jitter_core::models::ChatMessage {
    match event(line) {
        ChatEvent::Message(m) => m,
        other => unreachable!("expected Message, got {other:?}"),
    }
}

#[test]
fn tag_value_with_equals_is_kept_whole() {
    let m = parsed("@a=b=c;d=;e :tmi.twitch.tv NOTICE #x :hi");
    assert_eq!(m.tag("a"), Some("b=c"));
    assert_eq!(m.tag("d"), Some(""));
    assert_eq!(m.tag("e"), Some(""));
}

#[test]
fn tag_values_are_unescaped() {
    let m = parsed(r"@a=x\sy\:z\\w\nq;b=end\ :tmi.twitch.tv NOTICE #x :hi");
    assert_eq!(m.tag("a"), Some("x y;z\\w\nq"));
    assert_eq!(m.tag("b"), Some("end"));
}

#[test]
fn recorded_system_msg_is_unescaped() {
    let m = parsed(recorded("USERNOTICE sub"));
    assert_eq!(m.tag("system-msg"), Some("acaugu514 subscribed at Tier 1."));
}

#[test]
fn line_without_tags_parses() {
    let m = parsed(recorded("PING"));
    assert_eq!(m.command, "PING");
    assert!(m.tags.is_empty());
    assert_eq!(m.trailing.as_deref(), Some("tmi.twitch.tv"));

    let m = parsed(recorded("JOIN (own)"));
    assert_eq!(m.command, "JOIN");
    assert_eq!(m.params, ["#caedrel"]);
    assert_eq!(m.nick(), Some("justinfan7683"));
}

#[test]
fn reconnect_parses_and_is_not_an_event() {
    let m = parsed(":tmi.twitch.tv RECONNECT");
    assert_eq!(m.command, "RECONNECT");
    assert_eq!(to_event(&m), None);
    assert_eq!(to_event(&parsed(recorded("PING"))), None);
    assert_eq!(to_event(&parsed(recorded("CAP ACK"))), None);
}

#[test]
fn malformed_lines_give_none() {
    for line in ["", "\r\n", "@a=b", "@a=b ", ":prefix", "@a=b :prefix"] {
        assert_eq!(parse(line), None, "{line:?}");
    }
}

#[test]
fn privmsg_with_emotes_badges_and_color() {
    let m = message(recorded("PRIVMSG with emotes, badges, color"));
    assert_eq!(m.id, "17d2249e-1282-49cc-9e2e-20da4b009305");
    assert_eq!(m.sender.display_name, "Nightbot");
    assert_eq!(m.sender.user_id.as_str(), "19264788");
    assert_eq!(m.sender.color, Some(Rgb::new(0x7C, 0x7C, 0xE1)));
    let badges: Vec<_> = m
        .sender
        .badges
        .iter()
        .map(|b| (b.set_id.as_str(), b.version.as_str()))
        .collect();
    assert_eq!(
        badges,
        [("moderator", "1"), ("subscriber", "96"), ("bot-badge", "1")]
    );
    assert!(!m.action);
    assert_eq!(m.sent_at.unix(), 1793053809);
    let spans: Vec<_> = m
        .emotes
        .iter()
        .map(|s| (s.start, s.end, s.emote.name.as_str()))
        .collect();
    assert_eq!(spans, [(0, 10, "gotagaMoney"), (281, 289, "gotagaPog")]);
    assert_eq!(
        m.emotes[0].emote.url,
        "https://static-cdn.jtvnw.net/emoticons/v2/emotesv2_5ccaacdb3c4340908bfc69550dc97c85/default/dark/2.0"
    );
}

#[test]
fn emote_ranges_are_code_points() {
    let m = message(recorded(
        "PRIVMSG with emotes, badges, color; non-ASCII before an emote",
    ));
    assert_eq!(m.emotes.len(), 5);
    assert_eq!(m.emotes[0].emote.name, "PopNemo");
    assert_eq!((m.emotes[0].start, m.emotes[0].end), (55, 61));
}

#[test]
fn privmsg_without_color_badges_emotes() {
    let m = message(recorded("PRIVMSG no color, no badges, no emotes"));
    assert_eq!(m.sender.color, None);
    assert!(m.sender.badges.is_empty());
    assert!(m.emotes.is_empty());
    assert_eq!(m.text, "Classic LOST EVERYTHING");
}

#[test]
fn action_is_stripped_and_flagged() {
    let m = message(recorded("ACTION"));
    assert!(m.action);
    assert_eq!(m.text, "dviperSuit HypeHai raysLick whatifPls raysN");
    // Ranges index the text after the ACTION prefix.
    let names: Vec<_> = m.emotes.iter().map(|s| s.emote.name.as_str()).collect();
    assert_eq!(
        names,
        ["dviperSuit", "HypeHai", "raysLick", "whatifPls", "raysN"]
    );
}

#[test]
fn action_without_closing_marker_and_plain_text() {
    let line = "@id=a;user-id=1;display-name=N;tmi-sent-ts=5000;emotes= :n!n@n.tmi.twitch.tv PRIVMSG #c :\u{1}ACTION waves";
    let m = message(line);
    assert!(m.action);
    assert_eq!(m.text, "waves");
    // ACTION in the middle is just text.
    let line = "@id=a;user-id=1;display-name=N;tmi-sent-ts=5000 :n!n@n.tmi.twitch.tv PRIVMSG #c :ACTION no";
    let m = message(line);
    assert!(!m.action);
    assert_eq!(m.text, "ACTION no");
}

#[test]
fn out_of_range_emote_is_dropped() {
    let line = "@id=a;user-id=1;display-name=N;tmi-sent-ts=5000;emotes=25:0-2/26:4-9 :n!n@n.tmi.twitch.tv PRIVMSG #c :Kap";
    let m = message(line);
    assert_eq!(m.emotes.len(), 1);
    assert_eq!(m.emotes[0].emote.name, "Kap");
}

#[test]
fn privmsg_missing_required_tags_is_none() {
    let m = parsed("@display-name=N :n!n@n.tmi.twitch.tv PRIVMSG #c :hi");
    assert_eq!(to_event(&m), None);
}

#[test]
fn usernotice_with_text_is_a_message() {
    let m = message(recorded("USERNOTICE announcement"));
    assert_eq!(m.sender.display_name, "StreamElements");
    assert_eq!(
        m.text,
        "JOIN THE DISCORD: https://discord.com/invite/jynxzi"
    );
}

#[test]
fn usernotice_without_text_is_a_notice() {
    assert_eq!(
        event(recorded("USERNOTICE sub")),
        ChatEvent::Notice("acaugu514 subscribed at Tier 1.".into())
    );
    assert_eq!(
        event(recorded("USERNOTICE subgift")),
        ChatEvent::Notice("acaugu514 gifted a Tier 1 sub to mtdt1234!".into())
    );
}

#[test]
fn clearmsg_names_the_message() {
    assert_eq!(
        event(recorded("CLEARMSG")),
        ChatEvent::ClearMessage {
            id: "31bcc72f-36ee-4e65-b402-921cf95290d7".into()
        }
    );
}

#[test]
fn clearchat_with_target_clears_the_user() {
    let ChatEvent::ClearUser { user_id } = event(recorded("CLEARCHAT with target (timeout)"))
    else {
        unreachable!("expected ClearUser")
    };
    assert_eq!(user_id.as_str(), "1300688489");
}

#[test]
fn clearchat_without_target_clears_all() {
    let line = "@room-id=1;tmi-sent-ts=1 :tmi.twitch.tv CLEARCHAT #c";
    assert_eq!(event(line), ChatEvent::ClearAll);
}

#[test]
fn permanent_ban_clears_the_user() {
    let line = "@room-id=1;target-user-id=42;tmi-sent-ts=1 :tmi.twitch.tv CLEARCHAT #c :someone";
    let ChatEvent::ClearUser { user_id } = event(line) else {
        unreachable!("expected ClearUser")
    };
    assert_eq!(user_id.as_str(), "42");
}

#[test]
fn notice_carries_its_text() {
    let line = "@msg-id=slow_on :tmi.twitch.tv NOTICE #c :This room is now in slow mode.";
    assert_eq!(
        event(line),
        ChatEvent::Notice("This room is now in slow mode.".into())
    );
}

#[test]
fn full_roomstate_sets_all_fields() {
    assert_eq!(
        event(recorded("ROOMSTATE full (on join)")),
        ChatEvent::RoomState(RoomStateUpdate {
            emote_only: Some(false),
            subs_only: Some(false),
            r9k: Some(false),
            slow: Some(Seconds::new(12)),
        })
    );
}

#[test]
fn partial_roomstate_sets_only_present_fields() {
    let line = "@room-id=1;slow=30 :tmi.twitch.tv ROOMSTATE #c";
    assert_eq!(
        event(line),
        ChatEvent::RoomState(RoomStateUpdate {
            slow: Some(Seconds::new(30)),
            ..RoomStateUpdate::default()
        })
    );
}

#[test]
fn a_two_line_frame_yields_two_events() {
    let frame = format!(
        "{}\r\n{}\r\n",
        recorded("CLEARMSG"),
        recorded("CLEARCHAT with target (timeout)")
    );
    let events: Vec<_> = split_frame(&frame)
        .filter_map(parse)
        .filter_map(|m| to_event(&m))
        .collect();
    assert_eq!(events.len(), 2);
    assert!(matches!(events[0], ChatEvent::ClearMessage { .. }));
    assert!(matches!(events[1], ChatEvent::ClearUser { .. }));
}
