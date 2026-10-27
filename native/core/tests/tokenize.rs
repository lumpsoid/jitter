use jitter_core::models::{Emote, EmoteProvider, EmoteScope, EmoteSet, EmoteSpan, Fragment};
use jitter_core::tokenize::tokenize;

fn emote(name: &str, zero_width: bool) -> Emote {
    Emote {
        id: name.to_owned(),
        name: name.to_owned(),
        url: String::new(),
        still_url: None,
        zero_width,
    }
}

fn set_with(emotes: Vec<Emote>) -> EmoteSet {
    let mut set = EmoteSet::new();
    set.set(EmoteScope::Global, EmoteProvider::SevenTv, emotes);
    set
}

fn text(s: &str) -> Fragment {
    Fragment::Text(s.to_owned())
}

#[test]
fn twitch_ranges_are_code_points() {
    // The emoji is 4 UTF-8 bytes and 2 UTF-16 units but 1 code point, so
    // "Kappa" is 2-6 only when counting code points.
    let spans = [EmoteSpan {
        start: 2,
        end: 6,
        emote: emote("Kappa", false),
    }];
    assert_eq!(
        tokenize("😀 Kappa", &spans, &EmoteSet::new()),
        vec![text("😀 "), Fragment::Emote(emote("Kappa", false))]
    );
}

#[test]
fn text_around_and_between_spans_is_kept() {
    let spans = [
        EmoteSpan {
            start: 3,
            end: 4,
            emote: emote("aa", false),
        },
        EmoteSpan {
            start: 6,
            end: 7,
            emote: emote("bb", false),
        },
    ];
    assert_eq!(
        tokenize("hi aa bb!", &spans, &EmoteSet::new()),
        vec![
            text("hi "),
            Fragment::Emote(emote("aa", false)),
            text(" "),
            Fragment::Emote(emote("bb", false)),
            text("!"),
        ]
    );
}

#[test]
fn bad_spans_are_ignored() {
    let spans = [
        EmoteSpan {
            start: 0,
            end: 99,
            emote: emote("x", false),
        },
        EmoteSpan {
            start: 3,
            end: 1,
            emote: emote("y", false),
        },
    ];
    assert_eq!(
        tokenize("hello", &spans, &EmoteSet::new()),
        vec![text("hello")]
    );
}

#[test]
fn third_party_emotes_match_whole_words_only() {
    let set = set_with(vec![emote("LUL", false)]);
    assert_eq!(
        tokenize("LULW LUL xLUL", &[], &set),
        vec![
            text("LULW "),
            Fragment::Emote(emote("LUL", false)),
            text(" xLUL")
        ]
    );
}

#[test]
fn zero_width_after_an_emote_overlays_it() {
    let set = set_with(vec![
        emote("Base", false),
        emote("Hat", true),
        emote("Cape", true),
    ]);
    assert_eq!(
        tokenize("Base Hat Cape", &[], &set),
        vec![Fragment::Overlay {
            base: emote("Base", false),
            overlays: vec![emote("Hat", true), emote("Cape", true)],
        }]
    );
}

#[test]
fn zero_width_after_a_twitch_emote_overlays_it() {
    let set = set_with(vec![emote("Hat", true)]);
    let spans = [EmoteSpan {
        start: 0,
        end: 4,
        emote: emote("Kappa", false),
    }];
    assert_eq!(
        tokenize("Kappa Hat", &spans, &set),
        vec![Fragment::Overlay {
            base: emote("Kappa", false),
            overlays: vec![emote("Hat", true)],
        }]
    );
}

#[test]
fn zero_width_at_the_start_or_after_text_is_text() {
    let set = set_with(vec![emote("Hat", true)]);
    assert_eq!(tokenize("Hat", &[], &set), vec![text("Hat")]);
    assert_eq!(tokenize("hi Hat", &[], &set), vec![text("hi Hat")]);
}

#[test]
fn links_are_detected() {
    assert_eq!(
        tokenize(
            "see https://example.com/a?b=1 and www.x.org ok http://",
            &[],
            &EmoteSet::new()
        ),
        vec![
            text("see "),
            Fragment::Link("https://example.com/a?b=1".into()),
            text(" and "),
            Fragment::Link("www.x.org".into()),
            text(" ok http://"),
        ]
    );
}
