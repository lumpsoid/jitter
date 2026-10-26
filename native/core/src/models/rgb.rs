/// An sRGB colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Parses `#RRGGBB` (the `color` tag format). Anything else is `None`.
    pub fn parse_hex(input: &str) -> Option<Self> {
        let digits = input.strip_prefix('#')?;
        if digits.len() != 6 || !digits.is_ascii() {
            return None;
        }
        let channel = |i: usize| u8::from_str_radix(digits.get(i..i + 2)?, 16).ok();
        Some(Self::new(channel(0)?, channel(2)?, channel(4)?))
    }
}
