//! The Dart contract: one file per feature, DTOs shaped for the GUI.
//! Run `rinf gen` after changing anything here.
//!
//! `rinf gen` reads every signal type in the crate into one flat namespace,
//! so names are unique across files and avoid Flutter's own.

mod boot;
// Playback
mod playback;

pub use boot::{Boot, BootFailed, Booted};
// Playback
pub use playback::{
    LeavePlayer, Play, PlaybackTarget, PlaybackUpdate, PlayerCommand, PlayerPosition,
    QualityOption, SelectQuality,
};
