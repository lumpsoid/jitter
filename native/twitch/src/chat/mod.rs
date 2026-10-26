//! Twitch chat: the pure IRC line parser, the mapping of parsed lines to
//! core chat events, and the WebSocket connection that feeds them.

pub mod connection;
pub mod event;
pub mod irc;

pub use connection::connect;
pub use event::to_event;
pub use irc::{IrcMessage, parse, split_frame};
