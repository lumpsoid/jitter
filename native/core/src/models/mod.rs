//! Value objects and data models, one file each.

mod channel;
mod channel_id;
mod channel_login;
mod favorite;
mod page;
mod playback;
mod quality;
mod resume_point;
mod seconds;
mod stream;
mod timestamp;
mod variant;
mod video;
mod video_id;

pub use channel::Channel;
pub use channel_id::{ChannelId, ChannelIdError};
pub use channel_login::{ChannelLogin, ChannelLoginError};
pub use favorite::Favorite;
pub use page::{Cursor, Page};
pub use playback::Playback;
pub use quality::{Quality, QualityKind, Resolution};
pub use resume_point::ResumePoint;
pub use seconds::Seconds;
pub use stream::Stream;
pub use timestamp::Timestamp;
pub use variant::Variant;
pub use video::{Video, VideoKind};
pub use video_id::{VideoId, VideoIdError};
