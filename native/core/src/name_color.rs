//! Chat name colours: Twitch's default palette as the deterministic
//! fallback, and a per-theme clamp so every colour is readable.

use crate::models::{ChannelId, Rgb};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

impl Theme {
    fn background(self) -> Rgb {
        match self {
            Theme::Dark => Rgb::new(0x0E, 0x0E, 0x10),
            Theme::Light => Rgb::new(0xFF, 0xFF, 0xFF),
        }
    }

    fn toward(self) -> Rgb {
        match self {
            Theme::Dark => Rgb::new(0xFF, 0xFF, 0xFF),
            Theme::Light => Rgb::new(0x00, 0x00, 0x00),
        }
    }
}

/// WCAG contrast ratio a name must reach against the theme background.
pub const MIN_CONTRAST: f64 = 4.5;

/// Twitch's default name colours, used for users who never chose one.
pub const TWITCH_COLORS: [Rgb; 15] = [
    Rgb::new(0xFF, 0x00, 0x00),
    Rgb::new(0x00, 0x00, 0xFF),
    Rgb::new(0x00, 0x80, 0x00),
    Rgb::new(0xB2, 0x22, 0x22),
    Rgb::new(0xFF, 0x7F, 0x50),
    Rgb::new(0x9A, 0xCD, 0x32),
    Rgb::new(0xFF, 0x45, 0x00),
    Rgb::new(0x2E, 0x8B, 0x57),
    Rgb::new(0xDA, 0xA5, 0x20),
    Rgb::new(0xD2, 0x69, 0x1E),
    Rgb::new(0x5F, 0x9E, 0xA0),
    Rgb::new(0x1E, 0x90, 0xFF),
    Rgb::new(0xFF, 0x69, 0xB4),
    Rgb::new(0x8A, 0x2B, 0xE2),
    Rgb::new(0x00, 0xFF, 0x7F),
];

/// The colour to draw a chatter's name in: their own if set, else a fixed
/// pick from the user id, then moved toward the theme's text colour until it
/// is readable.
pub fn name_color(chosen: Option<Rgb>, user_id: &ChannelId, theme: Theme) -> Rgb {
    clamp(chosen.unwrap_or_else(|| fallback(user_id)), theme)
}

/// The same user id always maps to the same palette entry.
pub fn fallback(user_id: &ChannelId) -> Rgb {
    let index = fnv1a(user_id.as_str().as_bytes()) % TWITCH_COLORS.len() as u64;
    usize::try_from(index)
        .ok()
        .and_then(|i| TWITCH_COLORS.get(i))
        .copied()
        .unwrap_or(TWITCH_COLORS[0])
}

/// FNV-1a, 64 bit. Written out so the result never depends on a std hasher.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &b| {
        (hash ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn clamp(color: Rgb, theme: Theme) -> Rgb {
    let background = theme.background();
    let target = theme.toward();
    (0..=20u8)
        .map(|step| mix(color, target, step))
        .find(|c| contrast(*c, background) >= MIN_CONTRAST)
        .unwrap_or(target)
}

/// `step` twentieths of the way from `from` to `to`.
fn mix(from: Rgb, to: Rgb, step: u8) -> Rgb {
    let channel = |a: u8, b: u8| {
        let (a, b, step) = (i32::from(a), i32::from(b), i32::from(step));
        u8::try_from(a + (b - a) * step / 20).unwrap_or(b as u8)
    };
    Rgb::new(
        channel(from.r, to.r),
        channel(from.g, to.g),
        channel(from.b, to.b),
    )
}

fn luminance(c: Rgb) -> f64 {
    let lin = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b)
}

fn contrast(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}
