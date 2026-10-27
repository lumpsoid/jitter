//! Top streams: a grid of stream cards, the next page loading when the
//! grid is scrolled to its end.
//!
//! The loaded list only grows within one refresh, so a list that is still
//! loaded gets only its new streams appended; anything else starts the
//! grid over.

use std::cell::Cell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gdk, glib};
use jitter_core::models::{FirstPage, ListState, MorePages, Stream};
use tokio::runtime::Handle;

use crate::format::viewers_label;
use crate::run::{Event, send};

/// Thumbnail size requested for a card: 16:9, sharp at a card's width.
const THUMBNAIL_WIDTH: u32 = 440;
const THUMBNAIL_HEIGHT: u32 = 248;

/// Shown card size, in logical pixels.
const CARD_WIDTH: i32 = 320;
const CARD_HEIGHT: i32 = 180;

pub struct BrowsePage {
    page: adw::NavigationPage,
    stack: gtk::Stack,
    failed: adw::StatusPage,
    grid: gtk::FlowBox,
    more_loading: gtk::Spinner,
    more_failed: gtk::Box,
    more_message: gtk::Label,
    /// How many streams the grid shows, while it shows a loaded list.
    shown: Rc<Cell<Option<usize>>>,
    events: async_channel::Sender<Event>,
    thumbnails: Thumbnails,
}

impl BrowsePage {
    pub fn new(events: &async_channel::Sender<Event>, runtime: &Handle) -> Self {
        let grid = gtk::FlowBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .homogeneous(true)
            .min_children_per_line(1)
            .max_children_per_line(8)
            .column_spacing(12)
            .row_spacing(12)
            .margin_start(12)
            .margin_end(12)
            .margin_top(12)
            .valign(gtk::Align::Start)
            .build();

        let more_loading = gtk::Spinner::builder()
            .spinning(true)
            .visible(false)
            .margin_top(12)
            .margin_bottom(12)
            .build();
        let more_message = gtk::Label::new(None);
        let more_retry = gtk::Button::with_label("Retry");
        let more_failed = gtk::Box::builder()
            .spacing(12)
            .halign(gtk::Align::Center)
            .margin_top(12)
            .margin_bottom(12)
            .visible(false)
            .build();
        more_failed.append(&more_message);
        more_failed.append(&more_retry);

        let column = gtk::Box::new(gtk::Orientation::Vertical, 0);
        column.append(&grid);
        column.append(&more_loading);
        column.append(&more_failed);
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&column)
            .build();

        let loading = adw::Spinner::new();
        let failed_retry = gtk::Button::builder()
            .label("Retry")
            .halign(gtk::Align::Center)
            .css_classes(["pill", "suggested-action"])
            .build();
        let failed = adw::StatusPage::builder()
            .icon_name("network-error-symbolic")
            .title("Could not load the top streams")
            .child(&failed_retry)
            .build();
        let stack = gtk::Stack::new();
        stack.add_named(&loading, Some("loading"));
        stack.add_named(&failed, Some("failed"));
        stack.add_named(&scroll, Some("grid"));

        let refresh = gtk::Button::from_icon_name("view-refresh-symbolic");
        refresh.set_tooltip_text(Some("Refresh"));
        let header = adw::HeaderBar::new();
        header.pack_start(&refresh);
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&stack));
        let page = adw::NavigationPage::builder()
            .title("Top streams")
            .child(&toolbar)
            .build();

        for button in [&refresh, &failed_retry] {
            let events = events.clone();
            button.connect_clicked(move |_| send(&events, Event::RefreshBrowse));
        }
        let next = events.clone();
        more_retry.connect_clicked(move |_| send(&next, Event::NextBrowsePage));
        let next = events.clone();
        scroll.connect_edge_reached(move |_, edge| {
            if edge == gtk::PositionType::Bottom {
                send(&next, Event::NextBrowsePage);
            }
        });

        Self {
            page,
            stack,
            failed,
            grid,
            more_loading,
            more_failed,
            more_message,
            shown: Rc::new(Cell::new(None)),
            events: events.clone(),
            thumbnails: Thumbnails::new(runtime),
        }
    }

    pub fn page(&self) -> &adw::NavigationPage {
        &self.page
    }

    /// Shows `state`, appending only the streams the grid lacks while it
    /// stays loaded.
    pub fn show(&self, state: &ListState<Stream>) {
        match state {
            ListState::Idle | ListState::FirstPage(FirstPage::Loading) => {
                self.clear();
                self.stack.set_visible_child_name("loading");
            }
            ListState::FirstPage(FirstPage::Failed { message }) => {
                self.clear();
                self.failed.set_description(Some(message));
                self.stack.set_visible_child_name("failed");
            }
            ListState::Loaded { items, more, .. } => {
                let shown = self.shown.get().unwrap_or(0);
                for stream in items.get(shown..).unwrap_or_default() {
                    self.grid.append(&self.card(stream));
                }
                self.shown.set(Some(items.len()));
                self.show_more(more);
                self.stack.set_visible_child_name("grid");
            }
        }
    }

    fn clear(&self) {
        self.grid.remove_all();
        self.shown.set(None);
    }

    fn show_more(&self, more: &MorePages) {
        self.more_loading
            .set_visible(matches!(more, MorePages::Loading));
        self.more_failed
            .set_visible(matches!(more, MorePages::Failed { .. }));
        if let MorePages::Failed { message } = more {
            self.more_message.set_text(message);
        }
    }

    fn card(&self, stream: &Stream) -> gtk::Button {
        let picture = gtk::Picture::builder()
            .content_fit(gtk::ContentFit::Cover)
            .width_request(CARD_WIDTH)
            .height_request(CARD_HEIGHT)
            .css_classes(["card"])
            .build();
        self.thumbnails.load(
            &picture,
            stream.thumbnail(THUMBNAIL_WIDTH, THUMBNAIL_HEIGHT),
        );
        let title = label(&stream.title, &["heading"]);
        let name = label(&stream.channel.display_name, &[]);
        let about = match &stream.game {
            Some(game) => format!("{} · {}", game.name, viewers_label(stream.viewers)),
            None => viewers_label(stream.viewers),
        };
        let about = label(&about, &["dim-label", "caption"]);

        let content = gtk::Box::new(gtk::Orientation::Vertical, 4);
        content.append(&picture);
        content.append(&title);
        content.append(&name);
        content.append(&about);
        // Caps a card at its width: a title's natural width is its whole
        // text, which would leave one card per line.
        let clamp = adw::Clamp::builder()
            .maximum_size(CARD_WIDTH)
            .tightening_threshold(CARD_WIDTH)
            .child(&content)
            .build();
        let card = gtk::Button::builder()
            .child(&clamp)
            .css_classes(["flat"])
            .tooltip_text(&stream.title)
            .build();
        let (events, login) = (self.events.clone(), stream.channel.login.clone());
        card.connect_clicked(move |_| send(&events, Event::Play(login.clone())));
        card
    }
}

fn label(text: &str, classes: &[&str]) -> gtk::Label {
    gtk::Label::builder()
        .label(text)
        .xalign(0.0)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .max_width_chars(1)
        .hexpand(true)
        .css_classes(classes)
        .build()
}

/// Downloads card thumbnails on the network runtime. A thumbnail that fails
/// to load leaves its card blank.
struct Thumbnails {
    http: reqwest::Client,
    runtime: Handle,
}

impl Thumbnails {
    fn new(runtime: &Handle) -> Self {
        Self {
            http: reqwest::Client::new(),
            runtime: runtime.clone(),
        }
    }

    fn load(&self, picture: &gtk::Picture, url: String) {
        let http = self.http.clone();
        let download = self.runtime.spawn(async move {
            let response = http.get(url).send().await?.error_for_status()?;
            response.bytes().await
        });
        let picture = picture.downgrade();
        glib::spawn_future_local(async move {
            let Ok(Ok(bytes)) = download.await else {
                return;
            };
            let Ok(texture) = gdk::Texture::from_bytes(&glib::Bytes::from_owned(bytes)) else {
                return;
            };
            if let Some(picture) = picture.upgrade() {
                picture.set_paintable(Some(&texture));
            }
        });
    }
}
