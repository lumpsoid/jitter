//! The backup file: what the user keeps (favorites, settings, history and
//! VOD resume points) as JSON, to move to another machine or keep safe.
//! Values are as the database holds them: core's names for enums, Unix
//! seconds for times, whole seconds for positions.
//!
//! `version` is the file's layout. A reader takes any version up to its
//! own and refuses a newer one; a new field that older files lack is
//! optional, so they still read.

use jitter_core::models::{
    CategoryId, Channel, ChannelId, ChannelLogin, Favorite, GameRef, NewVisit, QualityPreference,
    ResumePoint, Seconds, Setting, Settings, ThemeChoice, Timestamp, VideoId, Visit, VisitDetails,
};
use jitter_core::playback::PlaybackTarget;
use jitter_core::ports::StoreError;
use serde::{Deserialize, Serialize};

/// Names the file as a backup, so other JSON is refused up front.
const FORMAT: &str = "jitter-backup";

/// The layout this build writes, and the newest it reads.
const VERSION: u32 = 1;

/// A backup's contents, read and checked: every value is one core accepts.
#[derive(Debug, Default)]
pub(crate) struct Contents {
    pub favorites: Vec<Favorite>,
    pub visits: Vec<NewVisit>,
    pub resume_points: Vec<ResumePoint>,
    /// The settings the file holds; one it lacks is not here.
    pub settings: Vec<Setting>,
}

/// The file for what is stored, as of `exported_at`.
pub(crate) fn write(
    exported_at: Timestamp,
    favorites: &[Favorite],
    visits: &[Visit],
    resume_points: &[ResumePoint],
    settings: &Settings,
) -> Result<String, StoreError> {
    let file = BackupFile {
        format: FORMAT.to_owned(),
        version: VERSION,
        exported_at: exported_at.unix(),
        settings: SettingsEntry::from(settings),
        favorites: favorites.iter().map(FavoriteEntry::from).collect(),
        history: visits.iter().map(VisitEntry::from).collect(),
        resume_points: resume_points.iter().map(ResumeEntry::from).collect(),
    };
    serde_json::to_string_pretty(&file)
        .map_err(|e| StoreError::new(format!("cannot write the backup: {e}")))
}

/// The contents of `json`. Fails, naming the first problem, unless it is
/// a backup of a version this build reads whose every value is valid.
pub(crate) fn read(json: &str) -> Result<Contents, StoreError> {
    let header: Header = serde_json::from_str(json).map_err(not_a_backup)?;
    if header.format != FORMAT {
        return Err(StoreError::new("not a Jitter backup"));
    }
    if header.version > VERSION {
        return Err(StoreError::new(format!(
            "the backup is version {}, newer than this app reads ({VERSION}); update Jitter",
            header.version
        )));
    }
    let file: BackupFile = serde_json::from_str(json).map_err(not_a_backup)?;
    Ok(Contents {
        favorites: file
            .favorites
            .into_iter()
            .map(FavoriteEntry::into_favorite)
            .collect::<Result<_, _>>()?,
        visits: file
            .history
            .into_iter()
            .map(VisitEntry::into_visit)
            .collect::<Result<_, _>>()?,
        resume_points: file
            .resume_points
            .into_iter()
            .map(ResumeEntry::into_point)
            .collect::<Result<_, _>>()?,
        settings: file.settings.into_settings()?,
    })
}

fn not_a_backup(error: serde_json::Error) -> StoreError {
    StoreError::new(format!("not a Jitter backup: {error}"))
}

fn invalid(what: &str, value: &str) -> StoreError {
    StoreError::new(format!("invalid {what} in the backup: {value:?}"))
}

/// Read first, so a newer layout is refused by its version and not by a
/// field this build does not know.
#[derive(Deserialize)]
struct Header {
    format: String,
    version: u32,
}

#[derive(Serialize, Deserialize)]
struct BackupFile {
    format: String,
    version: u32,
    exported_at: i64,
    #[serde(default)]
    settings: SettingsEntry,
    #[serde(default)]
    favorites: Vec<FavoriteEntry>,
    #[serde(default)]
    history: Vec<VisitEntry>,
    #[serde(default)]
    resume_points: Vec<ResumeEntry>,
}

/// Every setting is optional: a file from before a setting existed leaves
/// it as stored.
#[derive(Serialize, Deserialize, Default)]
struct SettingsEntry {
    theme: Option<String>,
    animated_emotes: Option<bool>,
    auto_load_chat: Option<bool>,
    start_quality: Option<String>,
    third_party_emotes: Option<bool>,
    background_audio: Option<bool>,
}

impl From<&Settings> for SettingsEntry {
    fn from(settings: &Settings) -> Self {
        Self {
            theme: Some(settings.theme.name().to_owned()),
            animated_emotes: Some(settings.animated_emotes),
            auto_load_chat: Some(settings.auto_load_chat),
            start_quality: Some(settings.start_quality.name().to_owned()),
            third_party_emotes: Some(settings.third_party_emotes),
            background_audio: Some(settings.background_audio),
        }
    }
}

impl SettingsEntry {
    fn into_settings(self) -> Result<Vec<Setting>, StoreError> {
        let theme = self
            .theme
            .map(|name| ThemeChoice::from_name(&name).ok_or_else(|| invalid("theme", &name)))
            .transpose()?;
        let start_quality = self
            .start_quality
            .map(|name| {
                QualityPreference::from_name(&name).ok_or_else(|| invalid("start quality", &name))
            })
            .transpose()?;
        Ok([
            theme.map(Setting::Theme),
            self.animated_emotes.map(Setting::AnimatedEmotes),
            self.auto_load_chat.map(Setting::AutoLoadChat),
            start_quality.map(Setting::StartQuality),
            self.third_party_emotes.map(Setting::ThirdPartyEmotes),
            self.background_audio.map(Setting::BackgroundAudio),
        ]
        .into_iter()
        .flatten()
        .collect())
    }
}

#[derive(Serialize, Deserialize)]
struct FavoriteEntry {
    channel_id: String,
    login: String,
    display_name: String,
    avatar_url: Option<String>,
    added_at: i64,
}

impl From<&Favorite> for FavoriteEntry {
    fn from(favorite: &Favorite) -> Self {
        Self {
            channel_id: favorite.channel_id.as_str().to_owned(),
            login: favorite.login.as_str().to_owned(),
            display_name: favorite.display_name.clone(),
            avatar_url: favorite.avatar_url.clone(),
            added_at: favorite.added_at.unix(),
        }
    }
}

impl FavoriteEntry {
    fn into_favorite(self) -> Result<Favorite, StoreError> {
        Ok(Favorite {
            channel_id: channel_id(&self.channel_id)?,
            login: login(&self.login)?,
            display_name: self.display_name,
            avatar_url: self.avatar_url,
            added_at: Timestamp::from_unix(self.added_at),
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum TargetKind {
    Live,
    Vod,
}

/// A visit without its id: the importing database gives it one.
#[derive(Serialize, Deserialize)]
struct VisitEntry {
    kind: TargetKind,
    /// A live channel's login, or a VOD's id.
    target: String,
    started_at: i64,
    last_seen_at: i64,
    title: Option<String>,
    channel: Option<ChannelEntry>,
    game: Option<GameEntry>,
    streamed_at: Option<i64>,
}

#[derive(Serialize, Deserialize)]
struct ChannelEntry {
    id: String,
    login: String,
    display_name: String,
    avatar_url: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct GameEntry {
    id: String,
    name: String,
}

impl From<&Visit> for VisitEntry {
    fn from(visit: &Visit) -> Self {
        let (kind, target) = match &visit.target {
            PlaybackTarget::Live(login) => (TargetKind::Live, login.as_str()),
            PlaybackTarget::Vod(id) => (TargetKind::Vod, id.as_str()),
        };
        let details = &visit.details;
        Self {
            kind,
            target: target.to_owned(),
            started_at: visit.started_at.unix(),
            last_seen_at: visit.last_seen_at.unix(),
            title: details.title.clone(),
            channel: details.channel.as_ref().map(|c| ChannelEntry {
                id: c.id.as_str().to_owned(),
                login: c.login.as_str().to_owned(),
                display_name: c.display_name.clone(),
                avatar_url: c.avatar().map(str::to_owned),
            }),
            game: details.game.as_ref().map(|g| GameEntry {
                id: g.id.as_str().to_owned(),
                name: g.name.clone(),
            }),
            streamed_at: details.streamed_at.map(Timestamp::unix),
        }
    }
}

impl VisitEntry {
    fn into_visit(self) -> Result<NewVisit, StoreError> {
        let target = match self.kind {
            TargetKind::Live => PlaybackTarget::Live(login(&self.target)?),
            TargetKind::Vod => PlaybackTarget::Vod(video_id(&self.target)?),
        };
        let channel = self
            .channel
            .map(|c| {
                Ok::<_, StoreError>(Channel {
                    id: channel_id(&c.id)?,
                    login: login(&c.login)?,
                    display_name: c.display_name,
                    avatar_url: c.avatar_url.unwrap_or_default(),
                })
            })
            .transpose()?;
        let game = self
            .game
            .map(|g| {
                Ok::<_, StoreError>(GameRef {
                    id: CategoryId::parse(&g.id).map_err(|_| invalid("category id", &g.id))?,
                    name: g.name,
                })
            })
            .transpose()?;
        Ok(NewVisit {
            target,
            started_at: Timestamp::from_unix(self.started_at),
            last_seen_at: Timestamp::from_unix(self.last_seen_at),
            details: VisitDetails {
                channel,
                title: self.title,
                game,
                streamed_at: self.streamed_at.map(Timestamp::from_unix),
            },
        })
    }
}

#[derive(Serialize, Deserialize)]
struct ResumeEntry {
    video_id: String,
    position_s: u32,
    duration_s: u32,
    saved_at: i64,
}

impl From<&ResumePoint> for ResumeEntry {
    fn from(point: &ResumePoint) -> Self {
        Self {
            video_id: point.video_id.as_str().to_owned(),
            position_s: point.position.get(),
            duration_s: point.duration.get(),
            saved_at: point.saved_at.unix(),
        }
    }
}

impl ResumeEntry {
    fn into_point(self) -> Result<ResumePoint, StoreError> {
        Ok(ResumePoint {
            video_id: video_id(&self.video_id)?,
            position: Seconds::new(self.position_s),
            duration: Seconds::new(self.duration_s),
            saved_at: Timestamp::from_unix(self.saved_at),
        })
    }
}

fn channel_id(raw: &str) -> Result<ChannelId, StoreError> {
    ChannelId::parse(raw).map_err(|_| invalid("channel id", raw))
}

fn login(raw: &str) -> Result<ChannelLogin, StoreError> {
    ChannelLogin::parse(raw).map_err(|_| invalid("channel login", raw))
}

fn video_id(raw: &str) -> Result<VideoId, StoreError> {
    VideoId::parse(raw).map_err(|_| invalid("video id", raw))
}
