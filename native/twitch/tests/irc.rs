//! IRC parser and event mapping. Lines under a `# <kind>` header of
//! `irc_lines.txt` are recorded from Twitch. Hand-written (Twitch did not
//! produce them in the recording session): partial ROOMSTATE, CLEARCHAT
//! without a target, permanent-ban CLEARCHAT, NOTICE, RECONNECT, tag values
//! containing `=`, `\:` and `\\`, the malformed lines, and the watch streak,
//! resub with text and raid USERNOTICEs.

use jitter_core::models::{ChatEvent, Rgb, RoomStateUpdate, Seconds, UserNotice};
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
    assert_eq!(
        m.emotes[0].emote.still_url.as_deref(),
        Some(
            "https://static-cdn.jtvnw.net/emoticons/v2/emotesv2_5ccaacdb3c4340908bfc69550dc97c85/static/dark/2.0"
        )
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

/// Hand-written in the shape Twitch sends (no such line came up while
/// recording): a watch streak, to which a test appends the user's text.
const WATCH_STREAK: &str = r"@badge-info=subscriber/5;badges=subscriber/3;color=#1E90FF;display-name=Riamot;emotes=;flags=;id=3d1b4f0e-6c1a-4f53-9b9e-0b5f8a3c1e21;login=riamot;mod=0;msg-id=viewermilestone;msg-param-category=watch-streak;msg-param-copoReward=450;msg-param-id=8f7e2b8c-5d3a-4a52-a0f1-2c4e9d6b7a10;msg-param-value=3;room-id=92038375;subscriber=1;system-msg=Riamot\swatched\s3\sconsecutive\sstreams\sand\ssparked\sa\swatch\sstreak!;tmi-sent-ts=1793053900000;user-id=403252242;user-type=;vip=0 :tmi.twitch.tv USERNOTICE #caedrel";

/// Hand-written: a resub with the user's text and an emote in it.
const RESUB_WITH_TEXT: &str = r"@badge-info=subscriber/11;badges=subscriber/9;color=#0000FF;display-name=Riamot;emotes=25:11-15;flags=;id=a4864201-4da1-466d-b11d-ea288b0d8e9c;login=riamot;mod=0;msg-id=resub;msg-param-cumulative-months=11;msg-param-months=0;msg-param-sub-plan=Prime;room-id=92038375;subscriber=1;system-msg=Riamot\ssubscribed\swith\sPrime.\sThey've\ssubscribed\sfor\s11\smonths!;tmi-sent-ts=1793053801834;user-id=403252242;user-type=;vip=0 :tmi.twitch.tv USERNOTICE #caedrel :still here Kappa";

/// Hand-written: a raid, which carries no text.
const RAID: &str = r"@badge-info=;badges=partner/1;color=#9ACD32;display-name=Raider;emotes=;flags=;id=5e1c7a3b-2f4d-4e6a-8b9c-1d2e3f4a5b6c;login=raider;mod=0;msg-id=raid;msg-param-displayName=Raider;msg-param-login=raider;msg-param-viewerCount=1234;room-id=92038375;subscriber=0;system-msg=1234\sraiders\sfrom\sRaider\shave\sjoined!;tmi-sent-ts=1793053950000;user-id=12826;user-type=;vip=0 :tmi.twitch.tv USERNOTICE #caedrel";

fn user_notice(line: &str) -> UserNotice {
    match event(line) {
        ChatEvent::UserNotice(n) => n,
        other => unreachable!("expected UserNotice, got {other:?}"),
    }
}

#[test]
fn usernotice_with_an_empty_system_msg_is_a_message() {
    let m = message(recorded("USERNOTICE announcement"));
    assert_eq!(m.sender.display_name, "StreamElements");
    assert_eq!(
        m.text,
        "JOIN THE DISCORD: https://discord.com/invite/jynxzi"
    );
}

#[test]
fn usernotice_without_text_is_a_bare_user_notice() {
    assert_eq!(
        user_notice(recorded("USERNOTICE sub")),
        UserNotice {
            system: "acaugu514 subscribed at Tier 1.".into(),
            message: None,
        }
    );
    assert_eq!(
        user_notice(recorded("USERNOTICE subgift")),
        UserNotice {
            system: "acaugu514 gifted a Tier 1 sub to mtdt1234!".into(),
            message: None,
        }
    );
    assert_eq!(
        user_notice(recorded("USERNOTICE resub (system-msg with \\s escapes)")).system,
        "Riamot subscribed with Prime. They've subscribed for 11 months!"
    );
}

#[test]
fn a_watch_streak_is_a_user_notice_not_the_rooms_notice() {
    let system = "Riamot watched 3 consecutive streams and sparked a watch streak!";
    assert_eq!(
        user_notice(WATCH_STREAK),
        UserNotice {
            system: system.into(),
            message: None,
        }
    );
    let with_text = user_notice(&format!("{WATCH_STREAK} :three in a row"));
    assert_eq!(with_text.system, system);
    let Some(m) = with_text.message else {
        unreachable!("the streak carries text");
    };
    assert_eq!(m.text, "three in a row");
    assert_eq!(m.id, "3d1b4f0e-6c1a-4f53-9b9e-0b5f8a3c1e21");
}

#[test]
fn a_resub_with_text_keeps_both_its_system_line_and_its_message() {
    let notice = user_notice(RESUB_WITH_TEXT);
    assert_eq!(
        notice.system,
        "Riamot subscribed with Prime. They've subscribed for 11 months!"
    );
    let Some(m) = notice.message else {
        unreachable!("the resub carries text");
    };
    assert_eq!(m.sender.display_name, "Riamot");
    assert_eq!(m.text, "still here Kappa");
    assert_eq!(m.emotes.len(), 1);
    assert_eq!(m.emotes[0].emote.name, "Kappa");
    assert_eq!((m.emotes[0].start, m.emotes[0].end), (11, 15));
}

#[test]
fn a_raid_is_a_bare_user_notice() {
    assert_eq!(
        user_notice(RAID),
        UserNotice {
            system: "1234 raiders from Raider have joined!".into(),
            message: None,
        }
    );
}

#[test]
fn usernotice_with_neither_system_msg_nor_text_is_none() {
    let line =
        "@id=a;user-id=1;display-name=N;tmi-sent-ts=5000;system-msg= :tmi.twitch.tv USERNOTICE #c";
    assert_eq!(to_event(&parsed(line)), None);
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
