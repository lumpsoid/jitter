//! Twitch chat: the pure IRC line parser and the mapping of parsed lines to
//! core chat events. The connection (sockets, timers) lives elsewhere.

pub mod event;
pub mod irc;

pub use event::to_event;
pub use irc::{IrcMessage, parse, split_frame};
