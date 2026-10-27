//! Channel lookup by login: a raw document written by us (no persisted
//! query for it; accepted anonymously, Phase 2). It also reads the user's
//! `description`, which only the channel page keeps:
//! [`Twitch::channel_profile`] returns it, [`Twitch::channel`] drops it.

use jitter_core::models::{Channel, ChannelLogin, ChannelProfile};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{RawQuery, UserNode};
use crate::client::Twitch;
use crate::error::TwitchError;

pub const QUERY: RawQuery = RawQuery {
    operation: "ChannelByLogin",
    document: "query($login: String!) { user(login: $login) { id login displayName \
               profileImageURL(width: 50) description } }",
};

pub fn variables(login: &ChannelLogin) -> Value {
    json!({ "login": login.as_str() })
}

/// Decodes a response body. An unknown login (`"user": null`) is
/// `NotFound`; a user core cannot represent is `Decode`. Twitch sends an
/// empty `description` for a channel without one: that is `None`.
pub fn parse(body: &[u8]) -> Result<ChannelProfile, TwitchError> {
    map(super::decode(body, QUERY.operation)?)
}

fn map(data: Data) -> Result<ChannelProfile, TwitchError> {
    let user = data.user.ok_or(TwitchError::NotFound)?;
    let channel = user
        .node
        .into_channel()
        .ok_or_else(|| TwitchError::Decode("ChannelByLogin: incomplete user".into()))?;
    Ok(ChannelProfile {
        channel,
        description: user.description.filter(|d| !d.trim().is_empty()),
    })
}

impl Twitch {
    /// The channel with this login.
    pub async fn channel(&self, login: &ChannelLogin) -> Result<Channel, TwitchError> {
        Ok(self.channel_profile(login).await?.channel)
    }

    /// The channel with this login, with its description.
    pub async fn channel_profile(
        &self,
        login: &ChannelLogin,
    ) -> Result<ChannelProfile, TwitchError> {
        map(self.query(&QUERY, variables(login)).await?)
    }
}

#[derive(Deserialize)]
struct Data {
    user: Option<User>,
}

#[derive(Deserialize)]
struct User {
    #[serde(flatten)]
    node: UserNode,
    description: Option<String>,
}
