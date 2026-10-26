//! Twitch access: API calls, playback tokens and playlists, chat. Turns
//! Twitch responses into core types and returns them; core never calls this
//! crate. No decisions; no rinf.

pub mod client;
pub mod error;
pub mod gql;
pub mod hls;
pub mod playback;

pub use client::Twitch;
pub use error::TwitchError;
