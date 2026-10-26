//! Business logic and state for jitter. No rinf, no tokio, no I/O: the
//! storage it needs enters through the ports defined here and implemented in
//! the store crate. Core never calls Twitch: twitch returns core types and
//! the heads hand them in. Heads pass the clock.

pub mod browse;
pub mod models;
pub mod playback;
