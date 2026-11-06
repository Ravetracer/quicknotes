#[macro_use]
mod i18n;
mod backup;
mod calc;
mod completion;
mod editor;
mod export;
mod format;
mod geocaching;
mod help;
mod html;
mod icons;
mod image_tab;
mod langs;
mod markdown;
mod plantuml;
mod presenter;
mod rst;
mod slides;
mod slides_assist;
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
.calc-results { border-left: 1px solid alpha(currentColor, 0.12); }
.font-group-start { border-top: 1px solid alpha(currentColor, 0.15); padding-top: 8px; margin-top: 4px; }
.image-canvas { background-color: alpha(currentColor, 0.04); }
.image-controls { border-radius: 10px; }
.image-controls label { padding: 0 4px; }

/* GtkSourceView hides the completion rows when a list runs empty and only
   refreshes them on the next frame; reopening it would then measure zero
   width, fail to present and leave stale pixels behind. */
GtkSourceAssistant.completion list { min-width: 240px; min-height: 24px; }

/* Tab colours by type (plain notes keep the default look). */
@define-color tab_code_color #2190a4;
tabbar tab.tab-code     { background-color: alpha(@tab_code_color, 0.14); }
tabbar tab.tab-document { background-color: alpha(@blue_3, 0.16); }
tabbar tab.tab-slides   { background-color: alpha(@purple_3, 0.16); }
tabbar tab.tab-calc     { background-color: alpha(@green_3, 0.16); }
tabbar tab.tab-image    { background-color: alpha(@orange_3, 0.16); }
tabbar tab.tab-code:selected     { background-color: alpha(@tab_code_color, 0.32); box-shadow: inset 0 -2px @tab_code_color; }
tabbar tab.tab-document:selected { background-color: alpha(@blue_3, 0.34); box-shadow: inset 0 -2px @blue_3; }
tabbar tab.tab-slides:selected   { background-color: alpha(@purple_3, 0.34); box-shadow: inset 0 -2px @purple_3; }
tabbar tab.tab-calc:selected     { background-color: alpha(@green_3, 0.34); box-shadow: inset 0 -2px @green_3; }
tabbar tab.tab-image:selected    { background-color: alpha(@orange_3, 0.34); box-shadow: inset 0 -2px @orange_3; }
"#;

fn main() -> glib::ExitCode {
    // Desktops like KDE set gtk-application-prefer-dark-theme for GTK apps.
    // libadwaita ignores it (the theme is set via AdwStyleManager) but warns
    // at startup; reset it for this process before libadwaita initializes.
    if gtk::init().is_ok() {
        if let Some(settings) = gtk::Settings::default() {
            settings.set_gtk_application_prefer_dark_theme(false);
        }
    }

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
