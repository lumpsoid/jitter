//! Channel lookup by login: a raw document written by us (no persisted
//! query for it; accepted anonymously, Phase 2).

use jitter_core::models::{Channel, ChannelLogin};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{RawQuery, UserNode};
use crate::client::Twitch;
use crate::error::TwitchError;

pub const QUERY: RawQuery = RawQuery {
    operation: "ChannelByLogin",
    document: "query($login: String!) { user(login: $login) { id login displayName \
               profileImageURL(width: 50) } }",
};

pub fn variables(login: &ChannelLogin) -> Value {
    json!({ "login": login.as_str() })
}

/// Decodes a response body. An unknown login (`"user": null`) is
/// `NotFound`; a user core cannot represent is `Decode`.
pub fn parse(body: &[u8]) -> Result<Channel, TwitchError> {
    map(super::decode(body, QUERY.operation)?)
}

fn map(data: Data) -> Result<Channel, TwitchError> {
    data.user
        .ok_or(TwitchError::NotFound)?
        .into_channel()
        .ok_or_else(|| TwitchError::Decode("ChannelByLogin: incomplete user".into()))
}

impl Twitch {
    /// The channel with this login.
    pub async fn channel(&self, login: &ChannelLogin) -> Result<Channel, TwitchError> {
        map(self.query(&QUERY, variables(login)).await?)
    }
}

#[derive(Deserialize)]
struct Data {
    user: Option<UserNode>,
}
