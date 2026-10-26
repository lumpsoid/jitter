//! Business logic and state for jitter. No rinf, no tokio, no I/O: the
//! infrastructure it needs (Twitch, storage, clock) enters through traits
//! defined here and implemented in the border crates.
