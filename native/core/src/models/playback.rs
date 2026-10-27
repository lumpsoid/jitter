use std::cmp::Reverse;

use super::{Quality, QualityKind, Variant};

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
