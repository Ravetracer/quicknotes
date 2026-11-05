mod editor;
mod format;
mod icons;
mod image_tab;
mod langs;
mod markdown;
mod plantuml;
mod store;
mod window;
mod xdnd;

use adw::prelude::*;
use gtk::{gdk, gio, glib};

pub const APP_ID: &str = "io.github.quicknotes";

const APP_CSS: &str = r#"
.statusbar {
    padding: 0 4px 0 10px;
    min-height: 30px;
    border-top: 1px solid alpha(currentColor, 0.12);
    font-size: 0.92em;
}
.statusbar label { opacity: 0.85; }
.statusbar button, .statusbar dropdown > button { min-height: 24px; padding: 0 8px; }
.search-panel {
    padding: 6px 8px;
    border-bottom: 1px solid alpha(currentColor, 0.12);
}
.image-canvas { background-color: alpha(currentColor, 0.04); }
.image-controls { border-radius: 10px; }
.image-controls label { padding: 0 4px; }
"#;

fn main() -> glib::ExitCode {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    app.connect_startup(|_| {
        langs::register();
        let display = gdk::Display::default().expect("no display");
        icons::register(&display);
        let css = gtk::CssProvider::new();
        css.load_from_string(APP_CSS);
        gtk::style_context_add_provider_for_display(&display, &css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    });
    app.connect_activate(|app| window::present(app, &[]));
    app.connect_open(|app, files, _| window::present(app, files));

    // Save everything when the session ends or the process is terminated.
    for signal in [libc_sig::SIGTERM, libc_sig::SIGINT, libc_sig::SIGHUP] {
        let app = app.clone();
        glib::unix_signal_add_local(signal, move || {
            window::persist_now();
            app.quit();
            glib::ControlFlow::Break
        });
    }

    app.run()
}

mod libc_sig {
    pub const SIGHUP: i32 = 1;
    pub const SIGINT: i32 = 2;
    pub const SIGTERM: i32 = 15;
}
