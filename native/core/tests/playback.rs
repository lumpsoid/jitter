use jitter_core::models::{ChannelLogin, Playback, QualityKind, Resolution, Variant, VideoId};
use jitter_core::playback::{PlaybackController, PlaybackState, PlaybackTarget};

fn live(login: &str) -> PlaybackTarget {
    let Ok(l) = ChannelLogin::parse(login) else {
        unreachable!("valid login");
    };
    PlaybackTarget::Live(l)
}

fn variant(name: &str, height: u32, kind: QualityKind) -> Variant {
    Variant {
        name: name.to_owned(),
        url: format!("https://example.test/{name}.m3u8"),
        resolution: Some(Resolution {
            width: height * 16 / 9,
            height,
        }),
        frame_rate: Some(30),
        bandwidth: 1000,
        kind,
    }
}

fn playback(variants: Vec<Variant>) -> Playback {
    Playback::from_variants("https://example.test/master.m3u8".to_owned(), variants)
}

fn selected(c: &PlaybackController) -> Option<&str> {
    c.selected_quality().map(|q| q.name.as_str())
}

#[test]
fn result_for_another_target_is_ignored() {
    let mut c = PlaybackController::new();
    c.begin(live("a"));
    c.begin(live("b"));
    c.apply(
        live("a"),
        Ok(playback(vec![variant("720p", 720, QualityKind::Transcode)])),
    );
    assert_eq!(c.state(), &PlaybackState::Resolving { target: live("b") });
    c.apply(live("a"), Err("late".to_owned()));
    assert_eq!(c.state(), &PlaybackState::Resolving { target: live("b") });
}

#[test]
fn result_while_idle_is_ignored() {
    let mut c = PlaybackController::new();
    c.apply(live("a"), Err("x".to_owned()));
    assert_eq!(c.state(), &PlaybackState::Idle);
}

#[test]
fn source_is_selected_by_default() {
    let mut c = PlaybackController::new();
    c.begin(live("a"));
    c.apply(
        live("a"),
        Ok(playback(vec![
            variant("720p", 720, QualityKind::Transcode),
            variant("1080p (source)", 1080, QualityKind::Source),
        ])),
    );
    assert_eq!(selected(&c), Some("1080p (source)"));
}

#[test]
fn no_source_selects_first_quality() {
    let mut c = PlaybackController::new();
    c.begin(live("a"));
    c.apply(
        live("a"),
        Ok(playback(vec![
            variant("480p", 480, QualityKind::Transcode),
            variant("720p", 720, QualityKind::Transcode),
        ])),
    );
    assert_eq!(selected(&c), Some("720p"));
}

#[test]
fn select_unknown_name_keeps_selection_and_known_name_changes_it() {
    let mut c = PlaybackController::new();
    let Ok(id) = VideoId::parse("123") else {
        unreachable!("valid id");
    };
    c.begin(PlaybackTarget::Vod(id.clone()));
    c.apply(
        PlaybackTarget::Vod(id),
        Ok(playback(vec![
            variant("720p", 720, QualityKind::Transcode),
            variant("480p", 480, QualityKind::Transcode),
        ])),
    );
    c.select("nope");
    assert_eq!(selected(&c), Some("720p"));
    c.select("480p");
    assert_eq!(selected(&c), Some("480p"));
}

#[test]
fn error_and_empty_playback_fail() {
    let mut c = PlaybackController::new();
    c.begin(live("a"));
    c.apply(live("a"), Err("offline".to_owned()));
    assert_eq!(
        c.state(),
        &PlaybackState::Failed {
            message: "offline".to_owned()
        }
    );
    c.begin(live("a"));
    c.apply(live("a"), Ok(playback(vec![])));
    assert!(matches!(c.state(), PlaybackState::Failed { .. }));
}
