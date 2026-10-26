//! The pure pieces of the chat connection: reconnect backoff, what an
//! incoming line asks for, and the client keepalive. PING, JOIN, CAP ACK,
//! 001 and PRIVMSG lines are recorded (`irc_lines.txt`); RECONNECT and the
//! PONG answer to our client PING are hand-written (Twitch did not produce
//! them in the recording session).

use std::time::{Duration, Instant};

use jitter_core::models::ChatEvent;
use jitter_twitch::chat::connection::{
    Action, BACKOFF_CAP, Keepalive, KeepaliveStep, PING_INTERVAL, PONG_TIMEOUT, Retry, backoff,
    next_action,
};

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

fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

#[test]
fn backoff_doubles_from_one_second_and_caps_at_sixty() {
    let schedule: Vec<u64> = (0..9).map(|n| backoff(n).as_secs()).collect();
    assert_eq!(schedule, [1, 2, 4, 8, 16, 32, 60, 60, 60]);
}

#[test]
fn backoff_stays_capped_for_huge_attempt_counts() {
    assert_eq!(backoff(31), BACKOFF_CAP);
    assert_eq!(backoff(32), BACKOFF_CAP);
    assert_eq!(backoff(u32::MAX), BACKOFF_CAP);
}

#[test]
fn retry_waits_longer_each_failure_and_resets_after_a_join() {
    let mut retry = Retry::default();
    let before: Vec<u64> = (0..4).map(|_| retry.on_failure().as_secs()).collect();
    assert_eq!(before, [1, 2, 4, 8]);
    retry.on_joined();
    assert_eq!(retry.on_failure(), secs(1));
    assert_eq!(retry.on_failure(), secs(2));
}

#[test]
fn server_ping_is_answered_with_the_same_token() {
    assert_eq!(recorded("PING"), "PING :tmi.twitch.tv");
    assert_eq!(
        next_action(recorded("PING")),
        Action::Reply("PONG :tmi.twitch.tv".to_owned())
    );
}

#[test]
fn bare_ping_gets_bare_pong() {
    assert_eq!(next_action("PING"), Action::Reply("PONG".to_owned()));
}

#[test]
fn reconnect_asks_for_a_reconnect() {
    assert_eq!(next_action(":tmi.twitch.tv RECONNECT"), Action::Reconnect);
}

#[test]
fn pong_to_our_ping_is_recognised() {
    assert_eq!(
        next_action(":tmi.twitch.tv PONG tmi.twitch.tv :tmi.twitch.tv"),
        Action::Pong
    );
}

#[test]
fn own_join_means_joined() {
    assert_eq!(next_action(recorded("JOIN (own)")), Action::Joined);
}

#[test]
fn chat_lines_become_events() {
    let action = next_action(recorded("PRIVMSG no color, no badges, no emotes"));
    assert!(matches!(action, Action::Event(e) if matches!(*e, ChatEvent::Message(_))));
}

#[test]
fn handshake_noise_and_malformed_lines_are_ignored() {
    assert_eq!(next_action(recorded("CAP ACK")), Action::Ignore);
    assert_eq!(next_action(recorded("welcome 001")), Action::Ignore);
    assert_eq!(next_action(""), Action::Ignore);
    assert_eq!(next_action("@only-tags"), Action::Ignore);
}

#[test]
fn keepalive_pings_after_the_interval() {
    let start = Instant::now();
    let mut keepalive = Keepalive::new(start);
    assert_eq!(keepalive.wake_at(), start + PING_INTERVAL);
    assert_eq!(keepalive.on_timer(start + PING_INTERVAL - secs(1)), None);
    assert_eq!(
        keepalive.on_timer(start + PING_INTERVAL),
        Some(KeepaliveStep::SendPing)
    );
    assert_eq!(keepalive.wake_at(), start + PING_INTERVAL + PONG_TIMEOUT);
}

#[test]
fn keepalive_times_out_without_a_pong() {
    let start = Instant::now();
    let mut keepalive = Keepalive::new(start);
    let pinged = start + PING_INTERVAL;
    assert_eq!(keepalive.on_timer(pinged), Some(KeepaliveStep::SendPing));
    assert_eq!(keepalive.on_timer(pinged + PONG_TIMEOUT - secs(1)), None);
    assert_eq!(
        keepalive.on_timer(pinged + PONG_TIMEOUT),
        Some(KeepaliveStep::TimedOut)
    );
}

#[test]
fn a_pong_restarts_the_interval() {
    let start = Instant::now();
    let mut keepalive = Keepalive::new(start);
    let pinged = start + PING_INTERVAL;
    assert_eq!(keepalive.on_timer(pinged), Some(KeepaliveStep::SendPing));
    let ponged = pinged + secs(1);
    keepalive.on_pong(ponged);
    assert_eq!(keepalive.wake_at(), ponged + PING_INTERVAL);
    assert_eq!(keepalive.on_timer(pinged + PONG_TIMEOUT), None);
}
