//! Top streams: the browse controller's actions and its state ↔ signals.
//!
//! Each action sends what changed since the last update Dart got: the
//! loaded list only grows within one refresh, so a loaded list that is
//! still loaded goes out as an `Appended` of its new streams, and anything
//! else as a `Snapshot`.

use jitter_core::browse::BrowseController;
use jitter_core::models::{FirstPage, ListState, MorePages, Page, PageRequest, Stream};
use jitter_twitch::Twitch;
use rinf::RustSignal;
use tokio::sync::mpsc::UnboundedSender;

use crate::format::viewers_label;
use crate::run::Fetched;
use crate::signals::{BrowseMore, BrowseState, BrowseUpdate, StreamCard};

/// Thumbnail size requested for a card: 16:9, sharp at a card's width on
/// a dense screen.
const THUMBNAIL_WIDTH: u32 = 440;
const THUMBNAIL_HEIGHT: u32 = 248;

/// `BrowseReady`: the whole state, for a holder that just subscribed.
pub fn ready(browse: &BrowseController) {
    BrowseUpdate::since(None, browse.state()).send_signal_to_dart();
}

/// `RefreshBrowse`, and the first load at start: starts over from the first
/// page and fetches it.
pub fn refresh(browse: &mut BrowseController, twitch: &Twitch, fetch: &UnboundedSender<Fetched>) {
    let shown = BrowseUpdate::shown(browse.state());
    let request = browse.begin_refresh();
    spawn_fetch(request, twitch, fetch);
    BrowseUpdate::since(shown, browse.state()).send_signal_to_dart();
}

/// `NextBrowsePage`: fetches the next page, unless core says there is none
/// to start now.
pub fn next(browse: &mut BrowseController, twitch: &Twitch, fetch: &UnboundedSender<Fetched>) {
    let shown = BrowseUpdate::shown(browse.state());
    if let Some(request) = browse.begin_next() {
        spawn_fetch(request, twitch, fetch);
        BrowseUpdate::since(shown, browse.state()).send_signal_to_dart();
    }
}

/// A fetched page, handed to core; core ignores a stale one.
pub fn on_page(
    browse: &mut BrowseController,
    request: PageRequest,
    result: Result<Page<Stream>, String>,
) {
    let shown = BrowseUpdate::shown(browse.state());
    browse.apply_page(request, result);
    BrowseUpdate::since(shown, browse.state()).send_signal_to_dart();
}

fn spawn_fetch(request: PageRequest, twitch: &Twitch, fetch: &UnboundedSender<Fetched>) {
    let (twitch, fetch) = (twitch.clone(), fetch.clone());
    tokio::spawn(async move {
        let result = twitch
            .top_streams(request.cursor.as_ref())
            .await
            .map_err(|e| e.to_string());
        let _ = fetch.send(Fetched::TopStreams { request, result });
    });
}

impl BrowseUpdate {
    /// How many streams Dart lists after the last update, when it lists a
    /// loaded page.
    fn shown(state: &ListState<Stream>) -> Option<usize> {
        match state {
            ListState::Loaded { items, .. } => Some(items.len()),
            ListState::Idle | ListState::FirstPage(_) => None,
        }
    }

    /// The update that takes Dart from `shown` streams to `state`.
    fn since(shown: Option<usize>, state: &ListState<Stream>) -> Self {
        match (shown, state) {
            (Some(shown), ListState::Loaded { items, more, .. }) => Self::Appended {
                streams: cards(items.get(shown..).unwrap_or_default()),
                more: more.into(),
            },
            _ => Self::Snapshot {
                state: state.into(),
            },
        }
    }
}

impl From<&ListState<Stream>> for BrowseState {
    fn from(state: &ListState<Stream>) -> Self {
        match state {
            ListState::Idle => Self::Idle,
            ListState::FirstPage(FirstPage::Loading) => Self::Loading,
            ListState::FirstPage(FirstPage::Failed { message }) => Self::Failed {
                message: message.clone(),
            },
            ListState::Loaded { items, more, .. } => Self::Loaded {
                streams: cards(items),
                more: more.into(),
            },
        }
    }
}

impl From<&MorePages> for BrowseMore {
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

fn cards(streams: &[Stream]) -> Vec<StreamCard> {
    streams.iter().map(StreamCard::from).collect()
}

impl From<&Stream> for StreamCard {
    fn from(stream: &Stream) -> Self {
        Self {
            id: stream.id.clone(),
            channel_id: stream.channel.id.as_str().to_owned(),
            login: stream.channel.login.as_str().to_owned(),
            display_name: stream.channel.display_name.clone(),
            title: stream.title.clone(),
            game_name: stream.game_name.clone(),
            viewers_label: viewers_label(stream.viewers),
            thumbnail_url: stream.thumbnail(THUMBNAIL_WIDTH, THUMBNAIL_HEIGHT),
        }
    }
}

#[cfg(test)]
mod tests {
    use jitter_core::models::{Channel, ChannelId, ChannelLogin, Cursor};

    use super::*;

    type TestResult = Result<(), String>;

    fn stream(id: &str) -> Result<Stream, String> {
        Ok(Stream {
            id: id.to_owned(),
            channel: Channel {
                id: ChannelId::parse("12").map_err(|e| format!("{e:?}"))?,
                login: ChannelLogin::parse("Streamer_1").map_err(|e| format!("{e:?}"))?,
                display_name: "Streamer_1".to_owned(),
                avatar_url: String::new(),
            },
            title: "Speedruns".to_owned(),
            game_name: Some("Celeste".to_owned()),
            viewers: 12_345,
            started_at: None,
            thumbnail_url: "https://cdn/{width}x{height}.jpg".to_owned(),
        })
    }

    fn page(ids: &[&str], next: Option<&str>) -> Result<Page<Stream>, String> {
        Ok(Page {
            items: ids.iter().map(|id| stream(id)).collect::<Result<_, _>>()?,
            next: next.map(Cursor::new),
        })
    }

    fn ids(streams: &[StreamCard]) -> Vec<&str> {
        streams.iter().map(|s| s.id.as_str()).collect()
    }

    /// A browse list with the first page `["a", "b"]` loaded.
    fn loaded() -> Result<BrowseController, String> {
        let mut browse = BrowseController::new();
        let request = browse.begin_refresh();
        browse.apply_page(request, Ok(page(&["a", "b"], Some("c1"))?));
        Ok(browse)
    }

    #[test]
    fn a_card_is_ready_to_show() -> TestResult {
        let card = StreamCard::from(&stream("s1")?);
        assert_eq!(card.channel_id, "12");
        assert_eq!(card.login, "streamer_1");
        assert_eq!(card.display_name, "Streamer_1");
        assert_eq!(card.viewers_label, "12.3K viewers");
        assert_eq!(card.thumbnail_url, "https://cdn/440x248.jpg");
        Ok(())
    }

    #[test]
    fn a_first_page_is_a_snapshot() -> TestResult {
        let mut browse = BrowseController::new();
        let request = browse.begin_refresh();
        let shown = BrowseUpdate::shown(browse.state());
        browse.apply_page(request, Ok(page(&["a", "b"], Some("c1"))?));
        let BrowseUpdate::Snapshot {
            state: BrowseState::Loaded { streams, more },
        } = BrowseUpdate::since(shown, browse.state())
        else {
            return Err("not a loaded snapshot".into());
        };
        assert_eq!(ids(&streams), ["a", "b"]);
        assert!(matches!(more, BrowseMore::Idle));
        Ok(())
    }

    #[test]
    fn starting_a_next_page_appends_only_the_footer() -> TestResult {
        let mut browse = loaded()?;
        let shown = BrowseUpdate::shown(browse.state());
        browse.begin_next().ok_or("no next page")?;
        let BrowseUpdate::Appended { streams, more } = BrowseUpdate::since(shown, browse.state())
        else {
            return Err("not appended".into());
        };
        assert!(streams.is_empty());
        assert!(matches!(more, BrowseMore::Loading));
        Ok(())
    }

    #[test]
    fn a_next_page_appends_only_its_new_streams() -> TestResult {
        let mut browse = loaded()?;
        let request = browse.begin_next().ok_or("no next page")?;
        let shown = BrowseUpdate::shown(browse.state());
        browse.apply_page(request, Ok(page(&["b", "c"], None)?));
        let BrowseUpdate::Appended { streams, more } = BrowseUpdate::since(shown, browse.state())
        else {
            return Err("not appended".into());
        };
        assert_eq!(ids(&streams), ["c"]);
        assert!(matches!(more, BrowseMore::Idle));
        Ok(())
    }

    #[test]
    fn a_failed_next_page_keeps_the_streams() -> TestResult {
        let mut browse = loaded()?;
        let request = browse.begin_next().ok_or("no next page")?;
        let shown = BrowseUpdate::shown(browse.state());
        browse.apply_page(request, Err("timed out".to_owned()));
        let BrowseUpdate::Appended { streams, more } = BrowseUpdate::since(shown, browse.state())
        else {
            return Err("not appended".into());
        };
        assert!(streams.is_empty());
        assert!(matches!(more, BrowseMore::Failed { message } if message == "timed out"));
        Ok(())
    }

    #[test]
    fn a_refresh_is_a_loading_snapshot() -> TestResult {
        let mut browse = loaded()?;
        let shown = BrowseUpdate::shown(browse.state());
        browse.begin_refresh();
        let update = BrowseUpdate::since(shown, browse.state());
        assert!(matches!(
            update,
            BrowseUpdate::Snapshot {
                state: BrowseState::Loading
            }
        ));
        Ok(())
    }

    #[test]
    fn a_failed_first_page_is_a_snapshot_of_its_message() -> TestResult {
        let mut browse = BrowseController::new();
        let request = browse.begin_refresh();
        browse.apply_page(request, Err("offline".to_owned()));
        let update = BrowseUpdate::since(None, browse.state());
        assert!(matches!(
            update,
            BrowseUpdate::Snapshot {
                state: BrowseState::Failed { message }
            } if message == "offline"
        ));
        Ok(())
    }
}
