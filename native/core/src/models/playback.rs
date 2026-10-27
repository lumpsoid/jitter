use std::cmp::Reverse;

use super::{Quality, QualityKind, QualityPreference, Variant};

/// A resolved stream: the master playlist and its qualities in display order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Playback {
    pub master_url: String,
    pub qualities: Vec<Quality>,
}

impl Playback {
    /// A name that means "the default selection". Real quality names are
    /// Twitch's `STABLE-VARIANT-ID`s (`1080p60`, `audio_only`), so the user
    /// needs a stable word for "the best one".
    pub const DEFAULT_QUALITY: &str = "source";

    /// Orders variants: source first, then transcodes by resolution, frame
    /// rate and bandwidth (all descending), then audio only.
    pub fn from_variants(master_url: String, mut variants: Vec<Variant>) -> Self {
        variants.sort_by_key(|v| {
            let rank = match v.kind {
                QualityKind::Source => 0,
                QualityKind::Transcode => 1,
                QualityKind::AudioOnly => 2,
            };
            (
                rank,
                Reverse(v.resolution),
                Reverse(v.frame_rate),
                Reverse(v.bandwidth),
            )
        });
        let qualities = variants
            .into_iter()
            .map(|v| Quality {
                name: v.name,
                url: v.url,
                resolution: v.resolution,
                frame_rate: v.frame_rate,
                kind: v.kind,
            })
            .collect();
        Self {
            master_url,
            qualities,
        }
    }

    /// The default selection: the source, else the first quality.
    pub fn default_quality(&self) -> Option<&Quality> {
        self.qualities
            .iter()
            .find(|q| q.kind == QualityKind::Source)
            .or_else(|| self.qualities.first())
    }

    /// The quality a page opens at for `preference`: the one asked for if
    /// the stream has it, else the closest below it, else the default
    /// selection. A height takes the tallest video quality not taller,
    /// at its highest frame rate; audio only takes the audio-only one.
    pub fn preferred_quality(&self, preference: QualityPreference) -> Option<&Quality> {
        let chosen = match preference {
            QualityPreference::Source => None,
            QualityPreference::AudioOnly => self
                .qualities
                .iter()
                .find(|q| q.kind == QualityKind::AudioOnly),
            _ => preference
                .height()
                .and_then(|height| self.tallest_up_to(height)),
        };
        chosen.or_else(|| self.default_quality())
    }

    /// The video quality with the greatest height not above `height`, then
    /// the greatest frame rate; on a tie, the earlier (the source first).
    /// Qualities without a known resolution are never picked.
    fn tallest_up_to(&self, height: u32) -> Option<&Quality> {
        self.qualities
            .iter()
            .filter(|q| q.kind != QualityKind::AudioOnly)
            .filter_map(|q| Some((q, q.resolution?.height)))
            .filter(|(_, h)| *h <= height)
            .fold(None, |best: Option<(&Quality, u32)>, (q, h)| match best {
                Some((b, bh)) if (bh, b.frame_rate) >= (h, q.frame_rate) => Some((b, bh)),
                _ => Some((q, h)),
            })
            .map(|(q, _)| q)
    }

    /// The quality called `name`, where [`Self::DEFAULT_QUALITY`] means the
    /// default selection unless a real quality has that name.
    pub fn quality(&self, name: &str) -> Option<&Quality> {
        self.qualities
            .iter()
            .find(|q| q.name == name)
            .or_else(|| match name {
                Self::DEFAULT_QUALITY => self.default_quality(),
                _ => None,
            })
    }
}
