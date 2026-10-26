use jitter_core::models::{ChannelId, Rgb};
use jitter_core::name_color::{TWITCH_COLORS, Theme, fallback, name_color};

fn id(raw: &str) -> ChannelId {
    let Ok(id) = ChannelId::parse(raw) else {
        unreachable!("valid id");
    };
    id
}

#[test]
fn the_same_user_id_always_gets_the_same_fallback() {
    assert_eq!(fallback(&id("12345")), fallback(&id("12345")));
    assert!(TWITCH_COLORS.contains(&fallback(&id("12345"))));
}

#[test]
fn the_fallback_is_pinned_to_the_hash() {
    // FNV-1a 64 of "1", mod 15, is 13. Pinned so a change of hash (and
    // every user's colour) shows up as a diff.
    assert_eq!(fallback(&id("1")), TWITCH_COLORS[13]);
}

#[test]
fn ids_spread_over_the_palette() {
    let distinct: std::collections::HashSet<_> =
        (1..200).map(|n| fallback(&id(&n.to_string()))).collect();
    assert!(distinct.len() >= 10);
}

#[test]
fn a_dark_colour_on_the_dark_theme_is_lightened() {
    let navy = Rgb::new(0, 0, 0x80);
    let shown = name_color(Some(navy), &id("1"), Theme::Dark);
    assert_ne!(shown, navy);
    assert!(shown.b > navy.b && shown.r > navy.r);
}

#[test]
fn a_light_colour_on_the_light_theme_is_darkened() {
    let yellow = Rgb::new(0xFF, 0xFF, 0x00);
    let shown = name_color(Some(yellow), &id("1"), Theme::Light);
    assert!(shown.r < yellow.r);
}

#[test]
fn a_readable_colour_is_unchanged() {
    let pink = Rgb::new(0xFF, 0x69, 0xB4);
    assert_eq!(name_color(Some(pink), &id("1"), Theme::Dark), pink);
}

#[test]
fn black_on_dark_still_becomes_readable() {
    let shown = name_color(Some(Rgb::new(0, 0, 0)), &id("1"), Theme::Dark);
    assert_ne!(shown, Rgb::new(0, 0, 0));
}

#[test]
fn hex_parsing() {
    assert_eq!(Rgb::parse_hex("#FF7F50"), Some(Rgb::new(0xFF, 0x7F, 0x50)));
    assert_eq!(Rgb::parse_hex(""), None);
    assert_eq!(Rgb::parse_hex("FF7F50"), None);
    assert_eq!(Rgb::parse_hex("#FF7F5"), None);
    assert_eq!(Rgb::parse_hex("#GG7F50"), None);
    assert_eq!(Rgb::parse_hex("#é7F50"), None);
}
