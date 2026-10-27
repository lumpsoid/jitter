//! The Dart contract: one file per feature, DTOs shaped for the GUI.
//! Run `rinf gen` after changing anything here.
//!
//! `rinf gen` reads every signal type in the crate into one flat namespace,
//! so names are unique across files and avoid Flutter's own.

mod boot;

pub use boot::{Boot, BootFailed, Booted};
