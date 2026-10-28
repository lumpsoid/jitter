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

fn video() -> VideoId {
    let Ok(id) = VideoId::parse("123") else {
        unreachable!("valid id");
    };
    id
}

fn vod() -> PlaybackTarget {
    PlaybackTarget::Vod(video())
}

#[test]
fn a_live_channel_joins_its_chat() {
    let mut c = ChatPanelController::new();
    assert_eq!(c.state(), ChatPanelState::Off);
    assert_eq!(
        c.play(Some(&live("xqc")), true),
        SessionChange::Join(login("xqc"))
    );
    assert_eq!(c.state(), ChatPanelState::Shown);
}

#[test]
fn a_vod_replays_its_chat() {
    let mut c = ChatPanelController::new();
    c.play(Some(&live("xqc")), true);
    assert_eq!(c.play(Some(&vod()), true), SessionChange::Replay(video()));
    assert_eq!(c.state(), ChatPanelState::Shown);
}

#[test]
fn a_rejected_target_has_no_chat() {
    let mut c = ChatPanelController::new();
    c.play(Some(&live("xqc")), true);
    assert_eq!(c.play(None, true), SessionChange::Leave);
    assert_eq!(c.state(), ChatPanelState::Off);
    c.play(Some(&vod()), true);
    assert_eq!(c.play(None, true), SessionChange::Leave);
    assert_eq!(c.state(), ChatPanelState::Off);
}

#[test]
fn hiding_a_replay_leaves_it_and_showing_replays_again() {
    let mut c = ChatPanelController::new();
    c.play(Some(&vod()), true);
    assert_eq!(c.hide(), SessionChange::Leave);
    assert_eq!(c.state(), ChatPanelState::Hidden);
    assert_eq!(c.show(), SessionChange::Replay(video()));
    assert_eq!(c.state(), ChatPanelState::Shown);
}

#[test]
fn a_chat_hidden_on_a_channel_stays_hidden_on_a_vod() {
    let mut c = ChatPanelController::new();
    c.play(Some(&live("xqc")), true);
    c.hide();
    assert_eq!(c.play(Some(&vod()), true), SessionChange::Leave);
    assert_eq!(c.state(), ChatPanelState::Hidden);
}

#[test]
fn hiding_leaves_the_chat_and_showing_rejoins_it() {
    let mut c = ChatPanelController::new();
    c.play(Some(&live("xqc")), true);
    assert_eq!(c.hide(), SessionChange::Leave);
    assert_eq!(c.state(), ChatPanelState::Hidden);
    assert_eq!(c.show(), SessionChange::Join(login("xqc")));
    assert_eq!(c.state(), ChatPanelState::Shown);
}

#[test]
fn repeating_hide_or_show_changes_nothing() {
    let mut c = ChatPanelController::new();
    c.play(Some(&live("xqc")), true);
    assert_eq!(c.show(), SessionChange::Keep);
    c.hide();
    assert_eq!(c.hide(), SessionChange::Keep);
    assert_eq!(c.state(), ChatPanelState::Hidden);
}

#[test]
fn a_hidden_chat_stays_hidden_on_the_next_channel() {
    let mut c = ChatPanelController::new();
    c.play(Some(&live("xqc")), true);
    c.hide();
    c.stop();
    assert_eq!(c.state(), ChatPanelState::Off);
    assert_eq!(c.play(Some(&live("shroud")), true), SessionChange::Leave);
    assert_eq!(c.state(), ChatPanelState::Hidden);
    assert_eq!(c.show(), SessionChange::Join(login("shroud")));
}

#[test]
fn stopping_leaves_the_chat() {
    let mut c = ChatPanelController::new();
    c.play(Some(&live("xqc")), true);
    assert_eq!(c.stop(), SessionChange::Leave);
    assert_eq!(c.state(), ChatPanelState::Off);
}

#[test]
fn showing_with_nothing_playing_joins_nothing() {
    let mut c = ChatPanelController::new();
    c.play(Some(&live("xqc")), true);
    c.hide();
    c.stop();
    assert_eq!(c.show(), SessionChange::Leave);
    assert_eq!(c.state(), ChatPanelState::Off);
    assert_eq!(
        c.play(Some(&live("xqc")), true),
        SessionChange::Join(login("xqc"))
    );
}

#[test]
fn without_auto_load_a_player_page_starts_hidden_and_joins_nothing() {
    let mut c = ChatPanelController::new();
    assert_eq!(c.play(Some(&live("xqc")), false), SessionChange::Leave);
    assert_eq!(c.state(), ChatPanelState::Hidden);
    assert_eq!(c.show(), SessionChange::Join(login("xqc")));
    assert_eq!(c.state(), ChatPanelState::Shown);
}

#[test]
fn without_auto_load_a_chat_shown_on_one_page_starts_hidden_on_the_next() {
    let mut c = ChatPanelController::new();
    c.play(Some(&live("xqc")), false);
    c.show();
    assert_eq!(c.play(Some(&vod()), false), SessionChange::Leave);
    assert_eq!(c.state(), ChatPanelState::Hidden);
}

#[test]
fn a_shown_chat_follows_the_page_into_its_recording_and_back() {
    let mut c = ChatPanelController::new();
    c.play(Some(&live("xqc")), true);
    assert_eq!(c.switch(&vod()), SessionChange::Replay(video()));
    assert_eq!(c.state(), ChatPanelState::Shown);
    assert_eq!(c.switch(&live("xqc")), SessionChange::Join(login("xqc")));
    assert_eq!(c.state(), ChatPanelState::Shown);
}

#[test]
fn a_hidden_chat_stays_hidden_through_a_switch() {
    let mut c = ChatPanelController::new();
    c.play(Some(&live("xqc")), false);
    assert_eq!(c.switch(&vod()), SessionChange::Leave);
    assert_eq!(c.state(), ChatPanelState::Hidden);
    assert_eq!(c.show(), SessionChange::Replay(video()));
}

#[test]
fn without_auto_load_a_switch_never_hides_a_shown_chat() {
    let mut c = ChatPanelController::new();
    c.play(Some(&live("xqc")), false);
    c.show();
    assert_eq!(c.switch(&vod()), SessionChange::Replay(video()));
    assert_eq!(c.state(), ChatPanelState::Shown);
}
