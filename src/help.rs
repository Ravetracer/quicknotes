//! In-app help window, rendered from `data/help.md`.

use crate::{html, markdown};
use adw::prelude::*;
use gtk::{gdk, gio};
use webkit6::prelude::*;

const HELP_EN: &str = include_str!("../data/help.md");
const HELP_DE: &str = include_str!("../data/help.de.md");

pub fn show(parent: &impl IsA<gtk::Window>, dark: bool, zoom: f64) {
    let help = match crate::i18n::lang() {
        crate::i18n::Lang::De => HELP_DE,
        crate::i18n::Lang::En => HELP_EN,
    };
    let body = markdown::to_html(help);
    let page = html::static_page(t!("QuickNotes Help"), &body, dark);

    let webview = webkit6::WebView::new();
    webview.set_vexpand(true);
    webview.set_zoom_level(zoom);
    webview.set_background_color(&if dark {
        gdk::RGBA::new(0x1e as f32 / 255.0, 0x1e as f32 / 255.0, 0x1e as f32 / 255.0, 1.0)
    } else {
        gdk::RGBA::WHITE
    });
    if let Some(settings) = WebViewExt::settings(&webview) {
        settings.set_enable_javascript(false);
    }
    // In-page links (#section) navigate within the help, others open in the browser.
    webview.connect_decide_policy(|_, decision, kind| {
        if kind != webkit6::PolicyDecisionType::NavigationAction {
            return false;
        }
        let Some(nav) = decision.downcast_ref::<webkit6::NavigationPolicyDecision>() else {
            return false;
        };
        let Some(mut action) = nav.navigation_action() else { return false };
        if action.navigation_type() != webkit6::NavigationType::LinkClicked {
            return false;
        }
        let uri = action.request().and_then(|r| r.uri()).unwrap_or_default();
        if uri.starts_with("about:blank#") {
            return false;
        }
        gtk::UriLauncher::new(&uri).launch(None::<&gtk::Window>, None::<&gio::Cancellable>, |_| {});
        decision.ignore();
        true
    });
    webview.load_html(&page, None);

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&webview));
    let window = adw::Window::builder()
        .title(t!("QuickNotes Help"))
        .default_width(880)
        .default_height(780)
        .transient_for(parent)
        .content(&toolbar)
        .build();
    window.present();
}
