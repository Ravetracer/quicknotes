//! The study window of flash cards: a web view showing one card after the
//! other (see `data/flashcards/study.js`). Answers are passed to the app as
//! they are given, so nothing is lost when the window is closed early.

use adw::prelude::*;
use gtk::glib;
use serde::Deserialize;
use std::cell::RefCell;
use std::rc::Rc;
use webkit6::prelude::*;

/// An answer given in the study window.
#[derive(Deserialize, Debug, Clone)]
pub struct Answer {
    pub key: String,
    pub rating: u8,
    pub ms: u32,
    /// The question (plain text).
    pub text: String,
    #[serde(default)]
    pub practice: bool,
}

pub struct Study {
    window: gtk::Window,
}

impl Study {
    /// Opens the study window with `page` (see [`crate::flashcards::study_page`]).
    pub fn open(
        parent: &impl IsA<gtk::Window>,
        title: &str,
        page: &str,
        base_uri: Option<String>,
        on_answer: impl Fn(Answer) + 'static,
        on_close: impl FnOnce() + 'static,
    ) -> Rc<Self> {
        let webview = webkit6::WebView::new();
        webview.set_hexpand(true);
        webview.set_vexpand(true);
        if let Some(s) = WebViewExt::settings(&webview) {
            s.set_enable_developer_extras(false);
            s.set_javascript_can_open_windows_automatically(false);
        }
        let window = gtk::Window::builder()
            .title(t!("Learning: {}", title))
            .default_width(1000)
            .default_height(760)
            .transient_for(parent)
            .child(&webview)
            .build();
        let this = Rc::new(Self { window: window.clone() });

        if let Some(ucm) = webview.user_content_manager() {
            ucm.register_script_message_handler("qn", None);
            let weak = Rc::downgrade(&this);
            ucm.connect_script_message_received(Some("qn"), move |_, value| {
                let Ok(msg) = serde_json::from_str::<serde_json::Value>(&value.to_str()) else { return };
                match msg.get("type").and_then(|v| v.as_str()) {
                    Some("rate") => {
                        if let Ok(answer) = serde_json::from_value::<Answer>(msg) {
                            on_answer(answer);
                        }
                    }
                    Some("close") => {
                        if let Some(s) = weak.upgrade() {
                            s.close();
                        }
                    }
                    Some("fullscreen") => {
                        if let Some(s) = weak.upgrade() {
                            if s.window.is_fullscreen() {
                                s.window.unfullscreen();
                            } else {
                                s.window.fullscreen();
                            }
                        }
                    }
                    _ => {}
                }
            });
        }
        let on_close = RefCell::new(Some(on_close));
        window.connect_close_request(move |_| {
            if let Some(f) = on_close.borrow_mut().take() {
                // After the window is gone, so the app sees the final state.
                glib::idle_add_local_once(f);
            }
            glib::Propagation::Proceed
        });
        webview.load_html(page, base_uri.as_deref());
        window.present();
        webview.grab_focus();
        this
    }

    pub fn close(&self) {
        self.window.close();
    }
}
