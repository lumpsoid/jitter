//! The rinf bridge: turns Dart signals into controller actions and
//! controller state into Rust signals. Entry point of the Rust logic.

mod signals;

use rinf::{dart_shutdown, write_interface};

write_interface!();

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Keep the main function running until Dart shutdown.
    dart_shutdown().await;
}
