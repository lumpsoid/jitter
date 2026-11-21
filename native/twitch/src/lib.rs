//! Twitch access: API calls, playback tokens and playlists, chat. Turns
//! Twitch responses into core types and returns them; core never calls this
//! crate. No decisions; no rinf. Also the part file a VOD download writes
//! into, so the heads share it with the segment fetch.

pub mod chat;
pub mod client;
pub mod emotes;
pub mod error;
pub mod gql;
pub mod hls;
pub mod part_file;
pub mod playback;

pub use client::Twitch;
pub use error::TwitchError;
