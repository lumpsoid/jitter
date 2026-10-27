//! The window: a navigation stack with the browse page at its root and the
//! player page pushed over it.

use adw::prelude::*;
use jitter_core::playback::PlaybackTarget;
use tokio::runtime::Handle;

use crate::browse::BrowsePage;
use crate::player::PlayerPage;
use crate::run::Event;

pub struct View {
    nav: adw::NavigationView,
    pub browse: BrowsePage,
    pub player: PlayerPage,
}

impl View {
    /// Builds the window and shows it. Fails when the player cannot start.
    pub fn new(
        app: &adw::Application,
        events: &async_channel::Sender<Event>,
        runtime: &Handle,
    ) -> Result<Self, String> {
        let browse = BrowsePage::new(events, runtime);
        let player = PlayerPage::new(events)?;
        let nav = adw::NavigationView::new();
        nav.add(browse.page());
        let leaving = player.clone();
        nav.connect_popped(move |_, page| {
            if page == leaving.page() {
                leaving.leave();
            }
        });
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Jitter")
            .default_width(1200)
            .default_height(760)
            .content(&nav)
            .build();
        window.present();
        Ok(Self {
            nav,
            browse,
            player,
        })
    }

    /// Shows the player page for `target`, pushing it unless it is up.
    pub fn open_player(&self, target: PlaybackTarget) {
        self.player.set_target(target);
        if self.nav.visible_page().as_ref() != Some(self.player.page()) {
            self.nav.push(self.player.page());
        }
    }
}
