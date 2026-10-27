use std::ops::{Add, Sub};

/// A non-negative duration or position in whole seconds. Arithmetic
/// saturates so policy math cannot underflow or overflow.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Seconds(u32);

impl Seconds {
    pub const ZERO: Self = Self(0);

    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub fn get(self) -> u32 {
        self.0
    }

    /// A time as the user types it: on a clock (`1:23:45`, `23:45`, or
    /// `45` seconds) or in units (`1h23m45s`, `1h 23m`, `90m`, `45s`, any
    /// of them, in that order). Only the first part may be 60 or more, as
    /// on a clock: `90:00` and `90m` are an hour and a half, `1:75` and
    /// `1h75m` are typos. Surrounding whitespace and the units' case do
    /// not matter.
    pub fn parse_clock(input: &str) -> Result<Self, ClockError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(ClockError::Empty);
        }
        let parts = if input.contains(':') {
            clock_parts(input)
        } else {
            unit_parts(input)
        }
        .ok_or(ClockError::Malformed)?;
        let mut total: u32 = 0;
        for (i, &(value, unit)) in parts.iter().enumerate() {
            if i > 0 && value >= 60 {
                return Err(ClockError::Malformed);
            }
            total = value
                .checked_mul(unit)
                .and_then(|part| total.checked_add(part))
                .ok_or(ClockError::Malformed)?;
        }
        Ok(Self(total))
    }

    /// A time to go to in a video `duration` long ([`Self::parse_clock`]),
    /// no further than its end. A zero `duration` is a video that has not
    /// loaded: any time goes.
    pub fn parse_position(input: &str, duration: Seconds) -> Result<Self, ClockError> {
        let at = Self::parse_clock(input)?;
        if duration > Self::ZERO && at > duration {
            return Err(ClockError::PastEnd { duration });
        }
        Ok(at)
    }
}

/// Why typed text is not a time: [`Seconds::parse_clock`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockError {
    Empty,
    Malformed,
    /// A time later than the end of the video, `duration` long.
    PastEnd {
        duration: Seconds,
    },
}

const MINUTE: u32 = 60;
const HOUR: u32 = 60 * MINUTE;

/// `h:m:s`, `m:s` or `s`, each part digits, as values with the seconds
/// each is worth, largest first.
fn clock_parts(input: &str) -> Option<Vec<(u32, u32)>> {
    let parts: Vec<&str> = input.split(':').collect();
    let units: &[u32] = match parts.len() {
        2 => &[MINUTE, 1],
        3 => &[HOUR, MINUTE, 1],
        _ => return None,
    };
    parts
        .iter()
        .zip(units)
        .map(|(part, &unit)| Some((digits(part)?, unit)))
        .collect()
}

/// Digits alone (seconds), or digits each followed by `h`, `m` or `s`,
/// each unit at most once and largest first, spaces allowed between.
fn unit_parts(input: &str) -> Option<Vec<(u32, u32)>> {
    if let Some(seconds) = digits(input) {
        return Some(vec![(seconds, 1)]);
    }
    let mut parts = Vec::new();
    let mut rest = input;
    let mut smaller_than = u32::MAX;
    while !rest.is_empty() {
        let end = rest.find(|c: char| !c.is_ascii_digit())?;
        let value = digits(rest.get(..end)?)?;
        let mut tail = rest.get(end..)?.chars();
        let unit = match tail.next()?.to_ascii_lowercase() {
            'h' => HOUR,
            'm' => MINUTE,
            's' => 1,
            _ => return None,
        };
        if unit >= smaller_than {
            return None;
        }
        smaller_than = unit;
        parts.push((value, unit));
        rest = tail.as_str().trim_start();
    }
    Some(parts)
}

/// A non-empty run of ASCII digits, no sign, as a number.
fn digits(input: &str) -> Option<u32> {
    if input.is_empty() || !input.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    input.parse().ok()
}

impl Add for Seconds {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl Sub for Seconds {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }
}
