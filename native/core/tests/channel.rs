use jitter_core::channel::{ChannelController, ChannelLookupError, ChannelState, LiveStatus};
use jitter_core::models::{Channel, ChannelId, ChannelLogin, ChannelProfile, Stream};

fn login(value: &str) -> ChannelLogin {
    let Ok(login) = ChannelLogin::parse(value) else {
        unreachable!("literal is valid");
    };
    login
}

fn id(value: &str) -> ChannelId {
    let Ok(id) = ChannelId::parse(value) else {
        unreachable!("literal is valid");
    };
    id
}

fn channel(channel_id: &str, name: &str) -> Channel {
    Channel {
        id: id(channel_id),
        login: login(name),
        display_name: name.to_owned(),
        avatar_url: String::new(),
    }
}

/// The channel page's lookup result: `channel` with no description.
fn profile(channel_id: &str, name: &str) -> ChannelProfile {
    ChannelProfile {
        channel: channel(channel_id, name),
        description: None,
    }
}

fn stream(channel_id: &str, name: &str) -> Stream {
    Stream {
        id: format!("s{channel_id}"),
        channel: channel(channel_id, name),
        title: "Live".to_owned(),
        game: None,
        viewers: 10,
        started_at: None,
        thumbnail_url: String::new(),
    }
}

/// A controller with `xqc` (id 1) found and its live check pending.
fn found() -> ChannelController {
    let mut c = ChannelController::new();
    c.open(login("xqc"));
    c.apply_channel(&login("xqc"), Ok(profile("1", "xqc")));
    c
}

fn live(c: &ChannelController) -> &LiveStatus {
    match c.state() {
        ChannelState::Found { live, .. } => live,
        other => unreachable!("not found: {other:?}"),
    }
}

#[test]
fn a_found_channel_asks_for_its_live_check() {
    let mut c = ChannelController::new();
    assert_eq!(c.state(), &ChannelState::Idle);
    c.open(login("xqc"));
    assert_eq!(
        c.state(),
        &ChannelState::Loading {
            login: login("xqc")
        }
    );
    let check = c.apply_channel(&login("xqc"), Ok(profile("1", "xqc")));
    assert_eq!(check, Some(id("1")));
    assert_eq!(live(&c), &LiveStatus::Checking);
}

#[test]
fn a_found_channel_keeps_its_description_through_its_live_check() {
    let mut c = ChannelController::new();
    c.open(login("xqc"));
    let about = ChannelProfile {
        channel: channel("1", "xqc"),
        description: Some("The juicer.".to_owned()),
    };
    c.apply_channel(&login("xqc"), Ok(about));
    c.apply_live(&id("1"), Ok(vec![]));
    assert_eq!(
        c.state(),
        &ChannelState::Found {
            channel: channel("1", "xqc"),
            description: Some("The juicer.".to_owned()),
            live: LiveStatus::Offline,
        }
    );
}

#[test]
fn an_unknown_login_is_not_found() {
    let mut c = ChannelController::new();
    c.open(login("nobody"));
    let check = c.apply_channel(&login("nobody"), Err(ChannelLookupError::NotFound));
    assert_eq!(check, None);
    assert_eq!(
        c.state(),
        &ChannelState::NotFound {
            login: login("nobody")
        }
    );
}

#[test]
fn a_failed_lookup_keeps_its_message() {
    let mut c = ChannelController::new();
    c.open(login("xqc"));
    c.apply_channel(
        &login("xqc"),
        Err(ChannelLookupError::Failed("offline".to_owned())),
    );
    assert_eq!(
        c.state(),
        &ChannelState::Failed {
            login: login("xqc"),
            message: "offline".to_owned()
        }
    );
}

#[test]
fn a_lookup_of_a_channel_no_longer_open_is_ignored() {
    let mut c = ChannelController::new();
    c.open(login("xqc"));
    c.open(login("shroud"));
    let check = c.apply_channel(&login("xqc"), Ok(profile("1", "xqc")));
    assert_eq!(check, None);
    assert_eq!(
        c.state(),
        &ChannelState::Loading {
            login: login("shroud")
        }
    );
}

#[test]
fn the_channels_own_stream_makes_it_live() {
    let mut c = found();
    c.apply_live(&id("1"), Ok(vec![stream("2", "other"), stream("1", "xqc")]));
    assert_eq!(live(&c), &LiveStatus::Live(Box::new(stream("1", "xqc"))));
}

#[test]
fn no_stream_is_offline_and_a_failed_check_keeps_its_message() {
    let mut c = found();
    c.apply_live(&id("1"), Ok(vec![]));
    assert_eq!(live(&c), &LiveStatus::Offline);

    let mut c = found();
    c.apply_live(&id("1"), Err("timed out".to_owned()));
    assert_eq!(
        live(&c),
        &LiveStatus::Failed {
            message: "timed out".to_owned()
        }
    );
}

#[test]
fn a_live_check_of_another_channel_is_ignored() {
    let mut c = found();
    c.apply_live(&id("2"), Ok(vec![stream("2", "other")]));
    assert_eq!(live(&c), &LiveStatus::Checking);
}

#[test]
fn refresh_looks_the_open_channel_up_again() {
    let mut c = ChannelController::new();
    assert_eq!(c.refresh(), None);

    let mut c = found();
    c.apply_live(&id("1"), Ok(vec![]));
    assert_eq!(c.refresh(), Some(login("xqc")));
    assert_eq!(
        c.state(),
        &ChannelState::Loading {
            login: login("xqc")
        }
    );
    // The live check from before the refresh no longer applies.
    c.apply_live(&id("1"), Ok(vec![stream("1", "xqc")]));
    assert!(matches!(c.state(), ChannelState::Loading { .. }));
}
