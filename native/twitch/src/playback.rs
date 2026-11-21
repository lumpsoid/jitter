//! Live and VOD playback through usher: token, URL, master playlist.

use std::time::Duration;

use futures_util::stream::{self, Stream, StreamExt};
use jitter_core::models::{ChannelLogin, Playback, SegmentList, VideoId};
use jitter_core::playback::PlaybackTarget;
use url::form_urlencoded;

use crate::client::Twitch;
use crate::error::TwitchError;
use crate::gql::playback_access_token::{AccessToken, Subject};
use crate::hls;

/// Usher hosts, live and VOD.
const USHER_LIVE: &str = "https://usher.ttvnw.net/api/v2/channel/hls/";
const USHER_VOD: &str = "https://usher.ttvnw.net/vod/v2/";

/// Codecs we ask for, best first.
const SUPPORTED_CODECS: &str = "av1,h265,h264";

/// Segment requests `segment_bytes` keeps in flight. A few hide each
/// request's round trip; many more would only crowd the link.
const SEGMENTS_IN_FLIGHT: usize = 4;
/// Requests per segment before `segment_bytes` gives up on it.
const SEGMENT_ATTEMPTS: u32 = 3;
/// The pause between two requests for one segment.
const RETRY_PAUSE: Duration = Duration::from_secs(1);

/// The nonce `p` is drawn from `0..NONCE_LIMIT`.
const NONCE_LIMIT: u32 = 9_999_999;

/// The usher URL for a live channel; `nonce` is the `p` parameter.
pub fn usher_live_url(login: &ChannelLogin, token: &AccessToken, nonce: u32) -> String {
    let query = query(&[("fast_bread", "true")], token, nonce);
    format!("{USHER_LIVE}{}.m3u8?{query}", login.as_str())
}

/// The usher URL for a VOD: the live parameters minus `fast_bread`.
pub fn usher_vod_url(id: &VideoId, token: &AccessToken, nonce: u32) -> String {
    let query = query(&[], token, nonce);
    format!("{USHER_VOD}{}.m3u8?{query}", id.as_str())
}

/// The shared parameters, percent-encoded, in the order Twitch echoes them.
/// `extra` goes after `allow_audio_only` (only live has `fast_bread`).
fn query(extra: &[(&str, &str)], token: &AccessToken, nonce: u32) -> String {
    let mut q = form_urlencoded::Serializer::new(String::new());
    q.append_pair("allow_source", "true")
        .append_pair("allow_audio_only", "true");
    q.extend_pairs(extra);
    q.append_pair("include_unavailable", "true")
        .append_pair("p", &nonce.to_string())
        .append_pair("platform", "web")
        .append_pair("sig", &token.signature)
        .append_pair("supported_codecs", SUPPORTED_CODECS)
        .append_pair("token", &token.value);
    q.finish()
}

/// What a non-success usher status means for `target`. Observed (Phase 2):
/// 404 on a live channel with a valid token is `Offline`
/// (`usher_live_offline.txt`); 403 is `PlaybackForbidden` (seen only for a
/// bad signature, `usher_live_bad_sig.txt`). A VOD 404 is `NotFound`.
pub fn usher_error(status: u16, target: &PlaybackTarget) -> TwitchError {
    match (status, target) {
        (404, PlaybackTarget::Live(_)) => TwitchError::Offline,
        (404, PlaybackTarget::Vod(_)) => TwitchError::NotFound,
        (403, _) => TwitchError::PlaybackForbidden,
        _ => TwitchError::Http { status },
    }
}

impl Twitch {
    /// Resolves a live channel into its qualities.
    pub async fn live(&self, login: &ChannelLogin) -> Result<Playback, TwitchError> {
        let token = self.access_token(Subject::Live(login)).await?;
        let url = usher_live_url(login, &token, nonce());
        self.master(url, &PlaybackTarget::Live(login.clone())).await
    }

    /// Resolves a VOD into its qualities.
    pub async fn vod(&self, id: &VideoId) -> Result<Playback, TwitchError> {
        let token = self.access_token(Subject::Vod(id)).await?;
        let url = usher_vod_url(id, &token, nonce());
        self.master(url, &PlaybackTarget::Vod(id.clone())).await
    }

    async fn master(&self, url: String, target: &PlaybackTarget) -> Result<Playback, TwitchError> {
        let (status, body) = self.get(&url).await?;
        if !(200..300).contains(&status) {
            return Err(usher_error(status, target));
        }
        Ok(Playback::from_variants(url, hls::parse_master(&body)?))
    }

    /// Resolves a VOD quality's media playlist into its segment URLs.
    pub async fn segments(&self, playlist_url: &str) -> Result<SegmentList, TwitchError> {
        let (status, body) = self.get(playlist_url).await?;
        if !(200..300).contains(&status) {
            return Err(TwitchError::Http { status });
        }
        hls::parse_media(&body, playlist_url)
    }

    /// The bytes of one media segment (or init segment) URL.
    pub async fn segment(&self, url: &str) -> Result<Vec<u8>, TwitchError> {
        let (status, bytes) = self.get(url).await?;
        if !(200..300).contains(&status) {
            return Err(TwitchError::Http { status });
        }
        Ok(bytes)
    }

    /// Each URL's bytes, in the order given, with up to `SEGMENTS_IN_FLIGHT`
    /// requests running at once. A failed request is tried again, up to
    /// `SEGMENT_ATTEMPTS` in all, because one transient error should not lose
    /// a download of ~1000 segments. Its last error is the stream's item.
    pub fn segment_bytes<'a>(
        &'a self,
        urls: &'a [String],
    ) -> impl Stream<Item = Result<Vec<u8>, TwitchError>> + 'a {
        stream::iter(urls)
            .map(|url| self.segment_retrying(url))
            .buffered(SEGMENTS_IN_FLIGHT)
    }

    async fn segment_retrying(&self, url: &str) -> Result<Vec<u8>, TwitchError> {
        let mut attempt = 1;
        loop {
            match self.segment(url).await {
                Ok(bytes) => return Ok(bytes),
                Err(e) if attempt == SEGMENT_ATTEMPTS => return Err(e),
                Err(_) => {
                    attempt += 1;
                    tokio::time::sleep(RETRY_PAUSE).await;
                }
            }
        }
    }
}

fn nonce() -> u32 {
    rand::random_range(0..NONCE_LIMIT)
}
