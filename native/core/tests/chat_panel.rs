use jitter_core::chat_panel::{ChatPanelController, ChatPanelState, SessionChange};
use jitter_core::models::{ChannelLogin, VideoId};
use jitter_core::playback::PlaybackTarget;

fn login(name: &str) -> ChannelLogin {
    let Ok(l) = ChannelLogin::parse(name) else {
        unreachable!("valid login");
    };
    l
}

fn live(name: &str) -> PlaybackTarget {
    PlaybackTarget::Live(login(name))
}

fn vod() -> PlaybackTarget {
    let Ok(id) = VideoId::parse("123") else {
        unreachable!("valid id");
    };
    PlaybackTarget::Vod(id)
}

#[test]
fn a_live_channel_joins_its_chat() {
    let mut c = ChatPanelController::new();
    assert_eq!(c.state(), ChatPanelState::Off);
    assert_eq!(
        c.play(Some(&live("xqc"))),
        SessionChange::Join(login("xqc"))
    );
    assert_eq!(c.state(), ChatPanelState::Shown);
}

#[test]
fn a_vod_or_a_rejected_target_has_no_chat() {
    let mut c = ChatPanelController::new();
    c.play(Some(&live("xqc")));
    assert_eq!(c.play(Some(&vod())), SessionChange::Leave);
    assert_eq!(c.state(), ChatPanelState::Off);
    c.play(Some(&live("xqc")));
    assert_eq!(c.play(None), SessionChange::Leave);
    assert_eq!(c.state(), ChatPanelState::Off);
}

#[test]
fn hiding_leaves_the_chat_and_showing_rejoins_it() {
    let mut c = ChatPanelController::new();
    c.play(Some(&live("xqc")));
    assert_eq!(c.hide(), SessionChange::Leave);
    assert_eq!(c.state(), ChatPanelState::Hidden);
    assert_eq!(c.show(), SessionChange::Join(login("xqc")));
    assert_eq!(c.state(), ChatPanelState::Shown);
}

#[test]
fn repeating_hide_or_show_changes_nothing() {
    let mut c = ChatPanelController::new();
    c.play(Some(&live("xqc")));
    assert_eq!(c.show(), SessionChange::Keep);
    c.hide();
    assert_eq!(c.hide(), SessionChange::Keep);
    assert_eq!(c.state(), ChatPanelState::Hidden);
}

#[test]
fn a_hidden_chat_stays_hidden_on_the_next_channel() {
    let mut c = ChatPanelController::new();
    c.play(Some(&live("xqc")));
    c.hide();
    c.stop();
    assert_eq!(c.state(), ChatPanelState::Off);
    assert_eq!(c.play(Some(&live("shroud"))), SessionChange::Leave);
    assert_eq!(c.state(), ChatPanelState::Hidden);
    assert_eq!(c.show(), SessionChange::Join(login("shroud")));
}

#[test]
fn stopping_leaves_the_chat() {
    let mut c = ChatPanelController::new();
    c.play(Some(&live("xqc")));
    assert_eq!(c.stop(), SessionChange::Leave);
    assert_eq!(c.state(), ChatPanelState::Off);
}

#[test]
fn showing_with_nothing_playing_joins_nothing() {
    let mut c = ChatPanelController::new();
    c.play(Some(&live("xqc")));
    c.hide();
    c.stop();
    assert_eq!(c.show(), SessionChange::Leave);
    assert_eq!(c.state(), ChatPanelState::Off);
    assert_eq!(
        c.play(Some(&live("xqc"))),
        SessionChange::Join(login("xqc"))
    );
}
