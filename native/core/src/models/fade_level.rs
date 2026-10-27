/// How loud the player plays relative to the user's own volume: 1.0 is the
/// user's volume, 0.0 silence. Clamped into that range at construction.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct FadeLevel(f64);

impl FadeLevel {
    /// The user's own volume: no fade.
    pub const FULL: Self = Self(1.0);

    /// `level` clamped to 0.0–1.0; a NaN is silence.
    pub fn new(level: f64) -> Self {
        if level.is_nan() {
            return Self(0.0);
        }
        Self(level.clamp(0.0, 1.0))
    }

    pub fn get(self) -> f64 {
        self.0
    }
}
