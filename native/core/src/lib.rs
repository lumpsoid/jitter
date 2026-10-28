//! Business logic and state for jitter. No rinf, no tokio, no I/O: the
//! storage it needs enters through the ports defined here and implemented in
//! the store crate. Core never calls Twitch: twitch returns core types and
//! the heads hand them in. Heads pass the clock.

pub mod browse;
pub mod categories;
pub mod category;
pub mod channel;
pub mod chat;
pub mod chat_panel;
pub mod chat_replay;
#[cfg(any(test, feature = "test-fakes"))]
pub mod fakes;
pub mod favorites;
pub mod history;
pub mod history_policy;
pub mod models;
pub mod name_color;
pub mod paged_list;
pub mod playback;
pub mod ports;
pub mod resume;
pub mod resume_policy;
pub mod rewind_policy;
pub mod search;
pub mod settings;
pub mod tokenize;
pub mod vods;
