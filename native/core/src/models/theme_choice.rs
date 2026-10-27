/// Which brightness the GUI draws in: the platform's, or one the user
/// fixed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeChoice {
    /// Follows the platform's light or dark mode.
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeChoice {
    /// Every choice, in the order a settings page offers them.
    pub const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];

    /// The stable name heads store and type: `system`, `light`, `dark`.
    pub fn name(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// The choice called `name` (see [`Self::name`]); `None` for any other
    /// text.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.name() == name)
    }
}
