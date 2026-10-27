//! Native Linux head: GTK4 and libadwaita on the same core, twitch and
//! store crates as the CLI and the Flutter hub. If it cannot do something
//! without Flutter, logic leaked into the hub or Dart.
//!
//! Widgets send [`run::Event`]s into one loop on the GTK main thread that
//! owns every controller; network calls run on a tokio runtime beside it
//! and come back on the same channel. Decisions stay in core.

mod browse;
mod format;
mod player;
mod run;
mod video;
mod view;

use std::cell::Cell;

use adw::prelude::*;
use gtk::glib;
use jitter_core::models::ChannelLogin;

const APP_ID: &str = "dev.jitter.Jitter";

/// `jitter-gtk [login]`: with a channel login, its stream opens at start.
fn main() -> glib::ExitCode {
    let watch = match std::env::args().nth(1).map(|a| ChannelLogin::parse(&a)) {
        None => None,
        Some(Ok(login)) => Some(login),
        Some(Err(e)) => return fail(&format!("not a channel login: {e:?}")),
    };
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(e) => return fail(&format!("starting the network runtime: {e}")),
    };
    let booted = match run::boot() {
        Ok(booted) => booted,
        Err(message) => return fail(&message),
    };
    let handle = runtime.handle().clone();
    let booted = Cell::new(Some(booted));
    let watch = Cell::new(watch);
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(move |app| {
        // A second activation (the app launched again) shows the window
        // the first one built.
        let Some((controllers, twitch)) = booted.take() else {
            if let Some(window) = app.active_window() {
                window.present();
            }
            return;
        };
        let (events, received) = async_channel::unbounded();
        let view = match view::View::new(app, &events, &handle) {
            Ok(view) => view,
            Err(message) => {
                eprintln!("jitter: {message}");
                app.quit();
                return;
            }
        };
        let context = run::Context {
            twitch,
            runtime: handle.clone(),
            events: events.clone(),
        };
        glib::spawn_future_local(run::run(controllers, context, received, view));
        run::send(&events, run::Event::RefreshBrowse);
        if let Some(login) = watch.take() {
            run::send(&events, run::Event::Play(login));
        }
    });
    // The arguments are this head's, not GApplication's.
    app.run_with_args::<&str>(&[])
}

fn fail(message: &str) -> glib::ExitCode {
    eprintln!("jitter: {message}");
    glib::ExitCode::FAILURE
}
