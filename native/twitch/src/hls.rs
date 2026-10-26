//! Master playlist to core `Variant`s. Real usher playlists (Phase 2
//! fixtures) have no `EXT-X-MEDIA`: the name is `STABLE-VARIANT-ID`, the
//! source is `IVS-VARIANT-SOURCE="source"`, audio is `STABLE-VARIANT-ID=
//! "audio_only"`. `m3u8-rs` keeps those unknown attributes in
//! `VariantStream::other_attributes`, so it is used as is.

use std::collections::HashMap;

use jitter_core::models::{QualityKind, Resolution, Variant};
use m3u8_rs::{QuotedOrUnquoted, VariantStream};

use crate::error::TwitchError;

const NAME_ATTRIBUTE: &str = "STABLE-VARIANT-ID";
const SOURCE_ATTRIBUTE: &str = "IVS-VARIANT-SOURCE";
const SOURCE_VALUE: &str = "source";
const AUDIO_ONLY_NAME: &str = "audio_only";

/// Parses a master playlist. Entries without a `STABLE-VARIANT-ID` (and
/// I-frame entries) are dropped; an unparsable playlist is `Decode`.
pub fn parse_master(body: &[u8]) -> Result<Vec<Variant>, TwitchError> {
    let playlist = m3u8_rs::parse_master_playlist_res(body)
        .map_err(|e| TwitchError::Decode(format!("master playlist: {e}")))?;
    Ok(playlist.variants.iter().filter_map(variant).collect())
}

fn variant(stream: &VariantStream) -> Option<Variant> {
    if stream.is_i_frame {
        return None;
    }
    let attributes = stream.other_attributes.as_ref()?;
    let name = attribute(attributes, NAME_ATTRIBUTE)?;
    let kind = if attribute(attributes, SOURCE_ATTRIBUTE) == Some(SOURCE_VALUE) {
        QualityKind::Source
    } else if name == AUDIO_ONLY_NAME {
        QualityKind::AudioOnly
    } else {
        QualityKind::Transcode
    };
    let resolution = stream.resolution.as_ref().and_then(|r| {
        Some(Resolution {
            width: u32::try_from(r.width).ok()?,
            height: u32::try_from(r.height).ok()?,
        })
    });
    Some(Variant {
        name: name.to_owned(),
        url: stream.uri.clone(),
        resolution,
        // Whole fps: 59.94 and 60.000 both display as 60.
        frame_rate: stream.frame_rate.map(|f| f.round() as u32),
        bandwidth: stream.bandwidth,
        kind,
    })
}

fn attribute<'a>(attributes: &'a HashMap<String, QuotedOrUnquoted>, key: &str) -> Option<&'a str> {
    match attributes.get(key)? {
        QuotedOrUnquoted::Quoted(s) | QuotedOrUnquoted::Unquoted(s) => Some(s),
    }
}
