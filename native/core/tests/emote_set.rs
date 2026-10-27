use jitter_core::models::{Emote, EmoteProvider, EmoteScope, EmoteSet};

fn emote(id: &str, name: &str) -> Emote {
    Emote {
        id: id.to_owned(),
        name: name.to_owned(),
        url: String::new(),
        still_url: None,
        zero_width: false,
    }
}

fn id_of(set: &EmoteSet, name: &str) -> Option<String> {
    set.get(name).map(|e| e.id.clone())
}

#[test]
fn channel_seventv_beats_a_global_emote_of_the_same_name() {
    let mut set = EmoteSet::new();
    set.set(
        EmoteScope::Global,
        EmoteProvider::SevenTv,
        vec![emote("g", "Kappa")],
    );
    set.set(
        EmoteScope::Global,
        EmoteProvider::Bttv,
        vec![emote("gb", "Kappa")],
    );
    set.set(
        EmoteScope::Channel,
        EmoteProvider::SevenTv,
        vec![emote("c", "Kappa")],
    );
    assert_eq!(id_of(&set, "Kappa").as_deref(), Some("c"));
}

#[test]
fn channel_ffz_beats_global_seventv() {
    let mut set = EmoteSet::new();
    set.set(
        EmoteScope::Global,
        EmoteProvider::SevenTv,
        vec![emote("g", "x")],
    );
    set.set(
        EmoteScope::Channel,
        EmoteProvider::Ffz,
        vec![emote("f", "x")],
    );
    assert_eq!(id_of(&set, "x").as_deref(), Some("f"));
}

#[test]
fn bttv_beats_ffz_and_seventv_beats_bttv() {
    let mut set = EmoteSet::new();
    set.set(
        EmoteScope::Channel,
        EmoteProvider::Ffz,
        vec![emote("f", "x")],
    );
    set.set(
        EmoteScope::Channel,
        EmoteProvider::Bttv,
        vec![emote("b", "x")],
    );
    assert_eq!(id_of(&set, "x").as_deref(), Some("b"));
    set.set(
        EmoteScope::Channel,
        EmoteProvider::SevenTv,
        vec![emote("s", "x")],
    );
    assert_eq!(id_of(&set, "x").as_deref(), Some("s"));
}

#[test]
fn setting_a_list_replaces_it_and_unknown_names_miss() {
    let mut set = EmoteSet::new();
    set.set(
        EmoteScope::Global,
        EmoteProvider::Ffz,
        vec![emote("a", "old")],
    );
    set.set(
        EmoteScope::Global,
        EmoteProvider::Ffz,
        vec![emote("b", "new")],
    );
    assert_eq!(id_of(&set, "old"), None);
    assert_eq!(id_of(&set, "new").as_deref(), Some("b"));
}
