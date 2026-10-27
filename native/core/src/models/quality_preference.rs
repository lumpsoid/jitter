/// The quality a player page opens at. A stream rarely offers every one,
/// so this is a preference: [`super::Playback::preferred_quality`] picks
/// what the stream has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QualityPreference {
    /// The best the stream offers: its source.
    #[default]
    Source,
    P1080,
    P720,
    P480,
    P360,
    P160,
    AudioOnly,
}

impl QualityPreference {
    /// Every preference, best first, in the order a settings page offers
    /// them.
    pub const ALL: [Self; 7] = [
        Self::Source,
        Self::P1080,
        Self::P720,
        Self::P480,
        Self::P360,
        Self::P160,
        Self::AudioOnly,
    ];

    /// The stable name heads store and type: `source`, `1080p` … `160p`,
    /// `audio_only`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::P1080 => "1080p",
            Self::P720 => "720p",
            Self::P480 => "480p",
            Self::P360 => "360p",
            Self::P160 => "160p",
            Self::AudioOnly => "audio_only",
        }
    }

    /// The preference called `name` (see [`Self::name`]); `None` for any
    /// other text.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|q| q.name() == name)
    }

    /// The video height a resolution preference asks for; `None` for the
    /// source and for audio only, which are not heights.
    pub fn height(self) -> Option<u32> {
        match self {
            Self::P1080 => Some(1080),
            Self::P720 => Some(720),
            Self::P480 => Some(480),
            Self::P360 => Some(360),
            Self::P160 => Some(160),
            Self::Source | Self::AudioOnly => None,
        }
    }
}
