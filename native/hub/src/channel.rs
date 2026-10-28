//! The channel page: the header controller and the VODs controller, their
//! calls, and their state ↔ signals.
//!
//! The header goes out whole after every change. The VODs go out like the
//! top streams: a loaded list that is still loaded goes out as an
//! `Appended` of its new VODs, anything else as a `Snapshot`. Each VOD
//! carries its resume progress, read from the store as it is sent.

use jitter_core::channel::{ChannelController, ChannelLookupError, ChannelState, LiveStatus};
use jitter_core::models::{
    ChannelId, ChannelLogin, ChannelProfile, FirstPage, ListState, MorePages, Page, PageRequest,
    Stream, Video, VideoKind,
};
use jitter_core::resume::ResumeController;
use jitter_core::vods::{FIRST_KIND, VodsController};
use jitter_twitch::{Twitch, TwitchError};
use rinf::RustSignal;
use tokio::sync::mpsc::UnboundedSender;

use crate::format::{duration_label, followers_label, streamed_label, twitch_error, viewers_label};
use crate::run::Fetched;
use crate::signals::{
    ChannelLive, ChannelUpdate, VodKind, VodTile, VodsMore, VodsState, VodsUpdate,
};

/// The open channel's controllers. The VODs controller is made by the
/// first `OpenChannel`: it needs a channel.
#[derive(Default)]
pub struct ChannelPage {
    header: ChannelController,
    vods: Option<VodsController>,
}

/// A channel page call's result, back in the loop as [`Fetched::Channel`].
pub enum ChannelFetched {
    /// The lookup an `OpenChannel` or `RefreshChannel` started.
    Channel {
        login: ChannelLogin,
        result: Result<ChannelProfile, TwitchError>,
    },
    /// The live check of a found channel.
    Live {
        id: ChannelId,
        result: Result<Vec<Stream>, TwitchError>,
    },
    /// A page of VODs.
    Vods {
        request: PageRequest,
        result: Result<Page<Video>, String>,
    },
}

/// `OpenChannel`: looks `login` up and lists its first kind of VODs. A
/// login that cannot be a channel's is not found.
pub fn open(
    page: &mut ChannelPage,
    resume: &ResumeController,
    login: &str,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
) {
    let Ok(login) = ChannelLogin::parse(login) else {
        let login = login.to_owned();
        return ChannelUpdate::NotFound { login }.send_signal_to_dart();
    };
    page.header.open(login.clone());
    spawn_lookup(login.clone(), twitch, fetch);
    send_header(&page.header);

    let vods = page
        .vods
        .get_or_insert_with(|| VodsController::new(login.clone(), FIRST_KIND));
    let request = vods.show(login, FIRST_KIND);
    spawn_vods(vods, request, twitch, fetch);
    VodsUpdate::since(None, vods.state(), resume).send_signal_to_dart();
}

/// `RefreshChannel`: looks the open channel up again and starts its VODs
/// over from the first page.
pub fn refresh(
    page: &mut ChannelPage,
    resume: &ResumeController,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
) {
    if let Some(login) = page.header.refresh() {
        spawn_lookup(login, twitch, fetch);
        send_header(&page.header);
    }
    if let Some(vods) = page.vods.as_mut() {
        let request = vods.begin_refresh();
        spawn_vods(vods, request, twitch, fetch);
        VodsUpdate::since(None, vods.state(), resume).send_signal_to_dart();
    }
}

/// `SelectVodKind`: lists the open channel's VODs of `kind` from the first
/// page, unless core says that kind is already listed.
pub fn select_kind(
    page: &mut ChannelPage,
    resume: &ResumeController,
    kind: VodKind,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
) {
    let Some(vods) = page.vods.as_mut() else {
        return;
    };
    if let Some(request) = vods.show_kind(kind.into()) {
        spawn_vods(vods, request, twitch, fetch);
        VodsUpdate::since(None, vods.state(), resume).send_signal_to_dart();
    }
}

/// `VodsNextPage`: fetches the next page, unless core says there is none to
/// start now.
pub fn next(
    page: &mut ChannelPage,
    resume: &ResumeController,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
) {
    let Some(vods) = page.vods.as_mut() else {
        return;
    };
    let shown = VodsUpdate::shown(vods.state());
    if let Some(request) = vods.begin_next() {
        spawn_vods(vods, request, twitch, fetch);
        VodsUpdate::since(shown, vods.state(), resume).send_signal_to_dart();
    }
}

/// Hands a channel page call's result to its controller, which ignores a
/// stale one, then sends that controller's state. A found channel's live
/// check starts here.
pub fn on_fetched(
    page: &mut ChannelPage,
    resume: &ResumeController,
    fetched: ChannelFetched,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
) {
    match fetched {
        ChannelFetched::Channel { login, result } => {
            let result = result.map_err(lookup_error);
            if let Some(id) = page.header.apply_channel(&login, result) {
                spawn_live(id, twitch, fetch);
            }
            send_header(&page.header);
        }
        ChannelFetched::Live { id, result } => {
            page.header
                .apply_live(&id, result.map_err(|e| twitch_error(&e)));
            send_header(&page.header);
        }
        ChannelFetched::Vods { request, result } => {
            if let Some(vods) = page.vods.as_mut() {
                let shown = VodsUpdate::shown(vods.state());
                vods.apply_page(request, result);
                VodsUpdate::since(shown, vods.state(), resume).send_signal_to_dart();
            }
        }
    }
}

fn lookup_error(error: TwitchError) -> ChannelLookupError {
    match error {
        TwitchError::NotFound => ChannelLookupError::NotFound,
        other => ChannelLookupError::Failed(twitch_error(&other)),
    }
}

fn spawn_lookup(login: ChannelLogin, twitch: &Twitch, fetch: &UnboundedSender<Fetched>) {
    let (twitch, fetch) = (twitch.clone(), fetch.clone());
    tokio::spawn(async move {
        let result = twitch.channel_profile(&login).await;
        let _ = fetch.send(Fetched::Channel(ChannelFetched::Channel { login, result }));
    });
}

fn spawn_live(id: ChannelId, twitch: &Twitch, fetch: &UnboundedSender<Fetched>) {
    let (twitch, fetch) = (twitch.clone(), fetch.clone());
    tokio::spawn(async move {
        let result = twitch.live_streams(std::slice::from_ref(&id)).await;
        let _ = fetch.send(Fetched::Channel(ChannelFetched::Live { id, result }));
    });
}

/// Fetches `request` for the channel and kind `vods` lists now.
fn spawn_vods(
    vods: &VodsController,
    request: PageRequest,
    twitch: &Twitch,
    fetch: &UnboundedSender<Fetched>,
) {
    let (channel, kind) = (vods.channel().clone(), vods.kind());
    let (twitch, fetch) = (twitch.clone(), fetch.clone());
    tokio::spawn(async move {
        let result = twitch
            .channel_videos(&channel, kind, request.cursor.as_ref())
            .await
            .map_err(|e| twitch_error(&e));
        let _ = fetch.send(Fetched::Channel(ChannelFetched::Vods { request, result }));
    });
}

fn send_header(header: &ChannelController) {
    ChannelUpdate::from(header.state()).send_signal_to_dart();
}

impl From<VodKind> for VideoKind {
    fn from(kind: VodKind) -> Self {
        match kind {
            VodKind::Archive => Self::Archive,
            VodKind::Highlight => Self::Highlight,
            VodKind::Upload => Self::Upload,
        }
    }
}

impl From<&ChannelState> for ChannelUpdate {
    fn from(state: &ChannelState) -> Self {
        match state {
            ChannelState::Idle => Self::Idle,
            ChannelState::Loading { login } => Self::Loading {
                login: login.as_str().to_owned(),
            },
            ChannelState::NotFound { login } => Self::NotFound {
                login: login.as_str().to_owned(),
            },
            ChannelState::Failed { login, message } => Self::Failed {
                login: login.as_str().to_owned(),
                message: message.clone(),
            },
            ChannelState::Found { profile, live } => Self::Found {
                channel_id: profile.channel.id.as_str().to_owned(),
                login: profile.channel.login.as_str().to_owned(),
                display_name: profile.channel.display_name.clone(),
                avatar_url: profile.channel.avatar_url.clone(),
                description: profile.description.clone(),
                followers_label: profile.followers.map(followers_label),
                live: live.into(),
            },
        }
    }
}

impl From<&LiveStatus> for ChannelLive {
    fn from(live: &LiveStatus) -> Self {
        match live {
            LiveStatus::Checking => Self::Checking,
            LiveStatus::Offline => Self::Offline,
            LiveStatus::Live(stream) => Self::Live {
                title: stream.title.clone(),
                game_name: stream.game.as_ref().map(|g| g.name.clone()),
                viewers_label: viewers_label(stream.viewers),
            },
            LiveStatus::Failed { message } => Self::Failed {
                message: message.clone(),
            },
        }
    }
}

impl VodsUpdate {
    /// How many VODs Dart lists after the last update, when it lists a
    /// loaded page.
    fn shown(state: &ListState<Video>) -> Option<usize> {
        match state {
            ListState::Loaded { items, .. } => Some(items.len()),
            ListState::Idle | ListState::FirstPage(_) => None,
        }
    }

    /// The update that takes Dart from `shown` VODs to `state`.
    fn since(shown: Option<usize>, state: &ListState<Video>, resume: &ResumeController) -> Self {
        match (shown, state) {
            (Some(shown), ListState::Loaded { items, more, .. }) => Self::Appended {
                vods: tiles(items.get(shown..).unwrap_or_default(), resume),
                more: more.into(),
            },
            _ => Self::Snapshot {
                state: VodsState::of(state, resume),
            },
        }
    }
}

impl VodsState {
    fn of(state: &ListState<Video>, resume: &ResumeController) -> Self {
        match state {
            ListState::Idle => Self::Idle,
            ListState::FirstPage(FirstPage::Loading) => Self::Loading,
            ListState::FirstPage(FirstPage::Failed { message }) => Self::Failed {
                message: message.clone(),
            },
            ListState::Loaded { items, more, .. } => Self::Loaded {
                vods: tiles(items, resume),
                more: more.into(),
            },
        }
    }
}

impl From<&MorePages> for VodsMore {
    fn from(more: &MorePages) -> Self {
        match more {
            MorePages::Idle => Self::Idle,
            MorePages::Loading => Self::Loading,
            MorePages::Failed { message } => Self::Failed {
                message: message.clone(),
            },
        }
    }
}

fn tiles(videos: &[Video], resume: &ResumeController) -> Vec<VodTile> {
    videos.iter().map(|v| VodTile::of(v, resume)).collect()
}

impl VodTile {
    /// A store that cannot be read shows no resume bar: the bar is a hint,
    /// and the player reports the failure when the VOD is opened.
    fn of(video: &Video, resume: &ResumeController) -> Self {
        Self {
            id: video.id.as_str().to_owned(),
            title: video.title.clone(),
            duration_label: duration_label(video.duration),
            date_label: streamed_label(video.published_at),
            thumbnail_url: video.thumbnail_url.clone(),
            progress: resume.progress(&video.id).ok().flatten(),
        }
    }
}

#[cfg(test)]
mod tests {
    use jitter_core::fakes::FakeResumeStore;
    use jitter_core::models::{Channel, Cursor, ResumePoint, Seconds, Timestamp, VideoId};

    use super::*;

    type TestResult = Result<(), String>;

    fn login() -> Result<ChannelLogin, String> {
        ChannelLogin::parse("xqc").map_err(|e| format!("{e:?}"))
    }

    fn video(id: &str) -> Result<Video, String> {
        Ok(Video {
            id: VideoId::parse(id).map_err(|e| format!("{e:?}"))?,
            channel: Channel {
                id: ChannelId::parse("12").map_err(|e| format!("{e:?}"))?,
                login: login()?,
                display_name: "xQc".to_owned(),
                avatar_url: String::new(),
            },
            title: "Reacting".to_owned(),
            duration: Seconds::new(3723),
            published_at: Timestamp::from_unix(0),
            thumbnail_url: "https://cdn/320x180.jpg".to_owned(),
            kind: VideoKind::Archive,
        })
    }

    fn page(ids: &[&str], next: Option<&str>) -> Result<Page<Video>, String> {
        Ok(Page {
            items: ids.iter().map(|id| video(id)).collect::<Result<_, _>>()?,
            next: next.map(Cursor::new),
        })
    }

    fn ids(vods: &[VodTile]) -> Vec<&str> {
        vods.iter().map(|v| v.id.as_str()).collect()
    }

    fn no_resume() -> ResumeController {
        ResumeController::new(Box::new(FakeResumeStore::default()))
    }

    /// VODs with the first page `["1", "2"]` loaded.
    fn loaded() -> Result<VodsController, String> {
        let mut vods = VodsController::new(login()?, FIRST_KIND);
        let request = vods.begin_refresh();
        vods.apply_page(request, Ok(page(&["1", "2"], Some("c1"))?));
        Ok(vods)
    }

    #[test]
    fn a_tile_is_ready_to_show_with_its_resume_progress() -> TestResult {
        let id = VideoId::parse("7").map_err(|e| format!("{e:?}"))?;
        let store = FakeResumeStore::with_points(vec![ResumePoint {
            video_id: id,
            position: Seconds::new(900),
            duration: Seconds::new(3600),
            saved_at: Timestamp::from_unix(0),
        }]);
        let resume = ResumeController::new(Box::new(store));
        let watched = VodTile::of(&video("7")?, &resume);
        assert_eq!(watched.duration_label, "1:02:03");
        assert_eq!(watched.date_label, streamed_label(Timestamp::from_unix(0)));
        assert_eq!(watched.progress, Some(0.25));
        let unwatched = VodTile::of(&video("8")?, &resume);
        assert_eq!(unwatched.progress, None);
        Ok(())
    }

    #[test]
    fn an_unreadable_store_shows_no_resume_bar() -> TestResult {
        let store = FakeResumeStore::default().failing_loads();
        let resume = ResumeController::new(Box::new(store));
        assert_eq!(VodTile::of(&video("7")?, &resume).progress, None);
        Ok(())
    }

    #[test]
    fn a_first_page_is_a_snapshot() -> TestResult {
        let vods = loaded()?;
        let VodsUpdate::Snapshot {
            state: VodsState::Loaded { vods, more },
        } = VodsUpdate::since(None, vods.state(), &no_resume())
        else {
            return Err("not a loaded snapshot".into());
        };
        assert_eq!(ids(&vods), ["1", "2"]);
        assert!(matches!(more, VodsMore::Idle));
        Ok(())
    }

    #[test]
    fn a_next_page_appends_only_its_new_vods() -> TestResult {
        let mut vods = loaded()?;
        let request = vods.begin_next().ok_or("no next page")?;
        let shown = VodsUpdate::shown(vods.state());
        vods.apply_page(request, Ok(page(&["2", "3"], None)?));
        let VodsUpdate::Appended { vods, more } =
            VodsUpdate::since(shown, vods.state(), &no_resume())
        else {
            return Err("not appended".into());
        };
        assert_eq!(ids(&vods), ["3"]);
        assert!(matches!(more, VodsMore::Idle));
        Ok(())
    }

    #[test]
    fn another_kind_is_a_loading_snapshot() -> TestResult {
        let mut vods = loaded()?;
        assert!(vods.show_kind(VodKind::Upload.into()).is_some());
        assert_eq!(vods.kind(), VideoKind::Upload);
        let update = VodsUpdate::since(None, vods.state(), &no_resume());
        assert!(matches!(
            update,
            VodsUpdate::Snapshot {
                state: VodsState::Loading
            }
        ));
        Ok(())
    }

    #[test]
    fn a_found_header_carries_the_description_and_followers() -> TestResult {
        let mut header = ChannelController::new();
        header.open(login()?);
        let channel = video("1")?.channel;
        let description = Some("The juicer.".to_owned());
        header.apply_channel(
            &login()?,
            Ok(ChannelProfile {
                channel,
                description,
                followers: Some(39_950),
            }),
        );
        let update = ChannelUpdate::from(header.state());
        assert!(matches!(
            update,
            ChannelUpdate::Found {
                description: Some(d),
                followers_label: Some(f),
                live: ChannelLive::Checking,
                ..
            } if d == "The juicer." && f == "39.9K followers"
        ));
        Ok(())
    }

    #[test]
    fn a_not_found_lookup_is_a_not_found_header() -> TestResult {
        let mut header = ChannelController::new();
        header.open(login()?);
        header.apply_channel(&login()?, Err(lookup_error(TwitchError::NotFound)));
        let update = ChannelUpdate::from(header.state());
        assert!(matches!(update, ChannelUpdate::NotFound { login } if login == "xqc"));
        Ok(())
    }

    #[test]
    fn another_lookup_failure_is_a_failed_header() -> TestResult {
        let mut header = ChannelController::new();
        header.open(login()?);
        let error = TwitchError::Http { status: 500 };
        header.apply_channel(&login()?, Err(lookup_error(error)));
        let update = ChannelUpdate::from(header.state());
        assert!(matches!(
            update,
            ChannelUpdate::Failed { message, .. }
                if message == "Twitch answered with an error (HTTP 500). Try again later."
        ));
        Ok(())
    }
}
