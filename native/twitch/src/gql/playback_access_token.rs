//! `PlaybackAccessToken`: the persisted query that yields the token and
//! signature usher wants, for a live channel or a VOD.

use jitter_core::models::{ChannelLogin, VideoId};
use serde::Deserialize;
use serde_json::{Value, json};

use super::PersistedQuery;
use crate::client::Twitch;
use crate::error::TwitchError;

/// The web client sends a fresh `X-Device-Id` with it, live and VOD alike.
pub const QUERY: PersistedQuery = PersistedQuery {
    operation: "PlaybackAccessToken",
    hash: "ed230aa1e33e07eebb8928504583da78a5173989fadfb1ac94be06a04f3cdbe9",
    needs_device_id: true,
};

/// What usher is asked to play.
#[derive(Debug, Clone, Copy)]
pub enum Subject<'a> {
    Live(&'a ChannelLogin),
    Vod(&'a VideoId),
}

/// A token that is allowed to play: its JSON `value` and `signature`, both
/// passed to usher verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessToken {
    pub value: String,
    pub signature: String,
}

/// Variables for the query; the fields the other subject does not use are
/// empty/false.
pub fn variables(subject: Subject<'_>) -> Value {
    let (is_live, login, is_vod, vod_id) = match subject {
        Subject::Live(login) => (true, login.as_str(), false, ""),
        Subject::Vod(id) => (false, "", true, id.as_str()),
    };
    json!({
        "isLive": is_live,
        "login": login,
        "isVod": is_vod,
        "vodID": vod_id,
        "playerType": "site",
        "platform": "web",
    })
}

/// Decodes a live token response. A null token (unknown login) is
/// `NotFound`; an offline channel still gets a normal token.
pub fn parse_live(body: &[u8]) -> Result<AccessToken, TwitchError> {
    let data: LiveData = super::decode(body, QUERY.operation)?;
    token(data.stream_playback_access_token)
}

/// Decodes a VOD token response. A null token (deleted or nonexistent VOD)
/// is `NotFound`.
pub fn parse_vod(body: &[u8]) -> Result<AccessToken, TwitchError> {
    let data: VodData = super::decode(body, QUERY.operation)?;
    token(data.video_playback_access_token)
}

/// `value` is itself JSON; its `authorization.forbidden` decides. Read
/// strictly: a value without it is `Decode`, not "allowed".
fn token(raw: Option<RawToken>) -> Result<AccessToken, TwitchError> {
    let raw = raw.ok_or(TwitchError::NotFound)?;
    let inner: TokenValue = serde_json::from_str(&raw.value)
        .map_err(|e| TwitchError::Decode(format!("PlaybackAccessToken value: {e}")))?;
    if inner.authorization.forbidden {
        return Err(TwitchError::PlaybackForbidden);
    }
    Ok(AccessToken {
        value: raw.value,
        signature: raw.signature,
    })
}

impl Twitch {
    pub(crate) async fn access_token(
        &self,
        subject: Subject<'_>,
    ) -> Result<AccessToken, TwitchError> {
        let variables = variables(subject);
        match subject {
            Subject::Live(_) => {
                let data: LiveData = self.persisted(&QUERY, variables).await?;
                token(data.stream_playback_access_token)
            }
            Subject::Vod(_) => {
                let data: VodData = self.persisted(&QUERY, variables).await?;
                token(data.video_playback_access_token)
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LiveData {
    stream_playback_access_token: Option<RawToken>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VodData {
    video_playback_access_token: Option<RawToken>,
}

#[derive(Deserialize)]
struct RawToken {
    value: String,
    signature: String,
}

#[derive(Deserialize)]
struct TokenValue {
    authorization: Authorization,
}

#[derive(Deserialize)]
struct Authorization {
    forbidden: bool,
}
