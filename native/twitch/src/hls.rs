//! Master playlist to core `Variant`s. Real usher playlists (Phase 2
//! fixtures) have no `EXT-X-MEDIA`: the name is `STABLE-VARIANT-ID`, the
//! source is `IVS-VARIANT-SOURCE="source"`, audio is `STABLE-VARIANT-ID=
//! "audio_only"`. `m3u8-rs` keeps those unknown attributes in
//! `VariantStream::other_attributes`, so it is used as is.
//!
//! Media playlists (one VOD quality's segments) to a core `SegmentList`.

use std::collections::HashMap;

use jitter_core::models::{QualityKind, Resolution, SegmentList, Variant};
use m3u8_rs::{QuotedOrUnquoted, VariantStream};
use url::Url;

use crate::error::TwitchError;

const NAME_ATTRIBUTE: &str = "STABLE-VARIANT-ID";
const SOURCE_ATTRIBUTE: &str = "IVS-VARIANT-SOURCE";
const SOURCE_VALUE: &str = "source";
const AUDIO_ONLY_NAME: &str = "audio_only";
const UNMUTED_SUFFIX: &str = "-unmuted.ts";
const MUTED_SUFFIX: &str = "-muted.ts";

/// Parses a master playlist. Entries without a `STABLE-VARIANT-ID` (and
/// I-frame entries) are dropped; an unparsable playlist is `Decode`.
pub fn parse_master(body: &[u8]) -> Result<Vec<Variant>, TwitchError> {
    let playlist = m3u8_rs::parse_master_playlist_res(body)
        .map_err(|e| TwitchError::Decode(format!("master playlist: {e}")))?;
    Ok(playlist.variants.iter().filter_map(variant).collect())
}

/// Parses a VOD media playlist into the absolute segment URLs, resolved
/// against `playlist_url`. A playlist without `EXT-X-ENDLIST` is accepted: it
/// is a VOD of a stream that is still going.
pub fn parse_media(body: &[u8], playlist_url: &str) -> Result<SegmentList, TwitchError> {
    let playlist = m3u8_rs::parse_media_playlist_res(body)
        .map_err(|e| TwitchError::Decode(format!("media playlist: {e}")))?;
    let base = Url::parse(playlist_url)
        .map_err(|e| TwitchError::Decode(format!("media playlist url: {e}")))?;
    let mut init: Option<String> = None;
    let mut segments = Vec::with_capacity(playlist.segments.len());
    for segment in &playlist.segments {
        if let Some(map) = &segment.map {
            let uri = resolve(&base, &map.uri)?;
            match &init {
                Some(first) if *first != uri => {
                    return Err(TwitchError::Decode(
                        "media playlist: more than one init segment".into(),
                    ));
                }
                Some(_) => {}
                None => init = Some(uri),
            }
        }
        segments.push(resolve(&base, &fetchable(&segment.uri))?);
    }
    if segments.is_empty() {
        return Err(TwitchError::Decode("media playlist: no segments".into()));
    }
    Ok(SegmentList { init, segments })
}

/// Twitch's `index-dvr` playlist lists a muted stretch as `<N>-unmuted.ts`,
/// which is not fetchable (403); `<N>-muted.ts` is. fMP4 needs no rewrite:
/// `<N>-unmuted.mp4` is fetchable, and the `index-muted-*` playlist the
/// usher resolves to lists `<N>-muted.ts` and `<N>-muted.mp4` already
/// (checked against the CDN, 2026-11-21).
fn fetchable(uri: &str) -> String {
    match uri.strip_suffix(UNMUTED_SUFFIX) {
        Some(stem) => format!("{stem}{MUTED_SUFFIX}"),
        None => uri.to_owned(),
    }
}

fn resolve(base: &Url, uri: &str) -> Result<String, TwitchError> {
    base.join(uri)
        .map(String::from)
        .map_err(|e| TwitchError::Decode(format!("media playlist uri {uri}: {e}")))
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
