use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::error::TwitchError;

/// Twitch's GQL endpoint.
pub const GQL_URL: &str = "https://gql.twitch.tv/gql";

/// The web client's id, which Twitch accepts for anonymous GQL when no
/// integrity token is sent.
pub const CLIENT_ID: &str = "kd1unb4b3q4t58fwlpcbzcbnm76a8fp";

const CLIENT_ID_HEADER: HeaderName = HeaderName::from_static("client-id");
const DEVICE_ID_HEADER: HeaderName = HeaderName::from_static("x-device-id");

/// The anonymous Twitch client. Cheap to clone; clones share one
/// connection pool.
#[derive(Debug, Clone)]
pub struct Twitch {
    http: reqwest::Client,
}

impl Twitch {
    pub fn new() -> Result<Self, TwitchError> {
        let http = reqwest::Client::builder()
            .build()
            .map_err(|e| TwitchError::Network(e.to_string()))?;
        Ok(Self { http })
    }

    /// POSTs a GQL body and returns the raw response body. A fresh device
    /// id is generated per request when `needs_device_id` is set.
    pub(crate) async fn post_gql(
        &self,
        body: &serde_json::Value,
        needs_device_id: bool,
    ) -> Result<Vec<u8>, TwitchError> {
        let device_id = needs_device_id.then(fresh_device_id);
        let response = self
            .http
            .post(GQL_URL)
            .headers(headers(device_id.as_deref())?)
            .json(body)
            .send()
            .await
            .map_err(|e| TwitchError::Network(e.to_string()))?;
        let status = response.status();
        if !status.is_success() {
            return Err(TwitchError::Http {
                status: status.as_u16(),
            });
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|e| TwitchError::Network(e.to_string()))?;
        Ok(bytes.to_vec())
    }
}

/// The headers of every GQL request; the only place they are built.
/// `Client-Id` always; `X-Device-Id` when the operation supplies one. No
/// `Authorization`: the client is anonymous (sign-in adds it here).
pub fn headers(device_id: Option<&str>) -> Result<HeaderMap, TwitchError> {
    let mut map = HeaderMap::new();
    map.insert(CLIENT_ID_HEADER, HeaderValue::from_static(CLIENT_ID));
    if let Some(id) = device_id {
        let value = HeaderValue::from_str(id)
            .map_err(|_| TwitchError::Decode(format!("invalid device id {id:?}")))?;
        map.insert(DEVICE_ID_HEADER, value);
    }
    Ok(map)
}

/// A random 32-hex-digit device id. `PlaybackAccessToken` sends a fresh one
/// per request when integrity is off, for live and VOD alike.
fn fresh_device_id() -> String {
    format!("{:032x}", rand::random::<u128>())
}
