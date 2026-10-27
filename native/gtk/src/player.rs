//! The player page: the video, what is playing, and the quality picker.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use jitter_core::models::{Open, Seconds};
use jitter_core::playback::{PlaybackState, PlaybackTarget};

use crate::run::{Event, send};
use crate::video::Video;

#[derive(Clone)]
pub struct PlayerPage {
    page: adw::NavigationPage,
    title: adw::WindowTitle,
    stack: gtk::Stack,
    failed: adw::StatusPage,
    qualities: gtk::StringList,
    quality: gtk::DropDown,
    /// Set while [`Self::show`] fills the picker, so that is not taken as
    /// the user picking.
    filling: Rc<Cell<bool>>,
    /// What the page plays, as the loop opened it; leaving reports it.
    target: Rc<RefCell<Option<PlaybackTarget>>>,
    video: Video,
    events: async_channel::Sender<Event>,
}

impl PlayerPage {
    pub fn new(events: &async_channel::Sender<Event>) -> Result<Self, String> {
        let video = Video::new()?;
        let loading = adw::Spinner::new();
        let failed = adw::StatusPage::builder()
            .icon_name("media-playback-stop-symbolic")
            .title("Could not play")
            .build();
        let stack = gtk::Stack::new();
        stack.add_named(&loading, Some("loading"));
        stack.add_named(&failed, Some("failed"));
        stack.add_named(video.widget(), Some("video"));

        let qualities = gtk::StringList::new(&[]);
        let quality = gtk::DropDown::builder()
            .model(&qualities)
            .tooltip_text("Quality")
            .sensitive(false)
            .build();
        let title = adw::WindowTitle::new("", "");
        let header = adw::HeaderBar::builder().title_widget(&title).build();
        header.pack_end(&quality);
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&stack));
        let page = adw::NavigationPage::builder()
            .title("Player")
            .child(&toolbar)
            .build();

        let filling = Rc::new(Cell::new(false));
        let (picking, filled, picked) = (events.clone(), filling.clone(), qualities.clone());
        quality.connect_selected_notify(move |quality| {
            if filled.get() {
                return;
            }
            if let Some(name) = picked.string(quality.selected()) {
                send(&picking, Event::SelectQuality(name.into()));
            }
        });

        Ok(Self {
            page,
            title,
            stack,
            failed,
            qualities,
            quality,
            filling,
            target: Rc::default(),
            video,
            events: events.clone(),
        })
    }

    pub fn page(&self) -> &adw::NavigationPage {
        &self.page
    }

    pub fn set_target(&self, target: PlaybackTarget) {
        self.target.replace(Some(target));
    }

    /// The page was popped: tells the loop which target it played.
    pub fn leave(&self) {
        if let Some(target) = self.target.take() {
            send(&self.events, Event::LeavePlayer(target));
        }
    }

    pub fn show(&self, state: &PlaybackState) {
        match state {
            PlaybackState::Idle => {}
            PlaybackState::Resolving { target } => {
                self.title.set_title(&target_name(target));
                self.title.set_subtitle("");
                self.quality.set_sensitive(false);
                self.stack.set_visible_child_name("loading");
            }
            PlaybackState::Ready {
                target,
                playback,
                selected,
                details,
                ..
            } => {
                let name = details
                    .channel
                    .as_ref()
                    .map_or_else(|| target_name(target), |c| c.display_name.clone());
                self.title.set_title(&name);
                let game = details.game.as_ref().map(|g| g.name.as_str());
                self.title.set_subtitle(game.unwrap_or_default());
                self.fill_qualities(playback.qualities.iter().map(|q| q.name.as_str()), selected);
                self.stack.set_visible_child_name("video");
            }
            PlaybackState::Slept { .. } => {
                self.quality.set_sensitive(false);
                self.failed.set_title("Stopped by the sleep timer");
                self.failed.set_description(None);
                self.stack.set_visible_child_name("failed");
            }
            PlaybackState::Failed { message } => {
                self.quality.set_sensitive(false);
                self.failed.set_title("Could not play");
                self.failed.set_description(Some(message));
                self.stack.set_visible_child_name("failed");
            }
        }
    }

    fn fill_qualities<'a>(&self, names: impl Iterator<Item = &'a str>, selected: &str) {
        self.filling.set(true);
        let names: Vec<&str> = names.collect();
        self.qualities.splice(0, self.qualities.n_items(), &names);
        if let Some(index) = names.iter().position(|n| *n == selected) {
            // Bounded by the picker's own list, which is far below u32::MAX.
            self.quality.set_selected(index as u32);
        }
        self.quality.set_sensitive(true);
        self.filling.set(false);
    }

    pub fn open(&self, open: &Open) {
        self.video.open(open);
    }

    pub fn stop(&self) {
        self.video.stop();
    }

    pub fn position(&self) -> Seconds {
        self.video.position()
    }
}

fn target_name(target: &PlaybackTarget) -> String {
    match target {
        PlaybackTarget::Live(login) => login.as_str().to_owned(),
        PlaybackTarget::Vod(id) => format!("VOD {}", id.as_str()),
    }
}
