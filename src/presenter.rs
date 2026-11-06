//! Running a presentation: an audience view and a presenter view (notes,
//! timer, next slide), each in its own web view.
//!
//! With two or more monitors both views are shown fullscreen on different
//! monitors: the presenter view where the main window is, the audience view
//! on another one. With one monitor (or in a window) both live in the same
//! window and `P` switches between them. Navigation in one view is
//! forwarded to the other one.

use crate::slides::{self, Deck, Mode, PageOptions};
use adw::prelude::*;
use gtk::{gdk, gio, glib};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use webkit6::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Layout {
    /// Fullscreen, on two monitors if available.
    Fullscreen,
    /// A normal window (rehearsal, preview).
    Window,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Audience,
    Presenter,
}

pub struct Presentation {
    app: adw::Application,
    audience: webkit6::WebView,
    presenter: webkit6::WebView,
    /// One window per screen; with a single window both views are in `stack`.
    windows: RefCell<Vec<gtk::Window>>,
    stack: Option<gtk::Stack>,
    /// Monitors of the audience and presenter window (two-screen mode).
    monitors: RefCell<Option<(gdk::Monitor, gdk::Monitor)>>,
    inhibit: Cell<u32>,
    ended: Cell<bool>,
    on_end: RefCell<Option<Box<dyn FnOnce()>>>,
}

impl Presentation {
    /// Starts the presentation at `start` (slide index). `on_end` runs once
    /// when it is closed.
    pub fn start(
        app: &adw::Application,
        parent: &impl IsA<gtk::Window>,
        deck: &Deck,
        base_uri: Option<String>,
        start: usize,
        layout: Layout,
        on_end: impl FnOnce() + 'static,
    ) -> Rc<Self> {
        let parent = parent.as_ref();
        let display = WidgetExt::display(parent);
        let monitors: Vec<gdk::Monitor> =
            display.monitors().iter::<gdk::Monitor>().filter_map(Result::ok).collect();
        let main_monitor = parent
            .surface()
            .and_then(|s| display.monitor_at_surface(&s))
            .or_else(|| monitors.first().cloned());
        let two_screens = layout == Layout::Fullscreen && monitors.len() > 1 && main_monitor.is_some();
        let screens = if two_screens { 2 } else { 1 };

        let audience = web_view();
        let presenter = web_view();
        let opts = PageOptions { start, screens, dark: true };
        audience.load_html(&slides::page(Mode::Audience, Some(deck), opts), base_uri.as_deref());
        presenter.load_html(&slides::page(Mode::Presenter, Some(deck), opts), base_uri.as_deref());

        let title = if deck.title.is_empty() { t!("Presentation").to_owned() } else { deck.title.clone() };
        let new_window = |child: &gtk::Widget, title: &str| {
            let win = gtk::Window::builder()
                .application(app)
                .title(title)
                .icon_name(crate::APP_ID)
                .default_width(1280)
                .default_height(760)
                .child(child)
                .build();
            win.add_css_class("presentation");
            win
        };

        let (windows, stack, pair) = if two_screens {
            let presenter_monitor = main_monitor.clone().unwrap_or_else(|| monitors[0].clone());
            let audience_monitor = monitors
                .iter()
                .find(|m| **m != presenter_monitor)
                .cloned()
                .unwrap_or_else(|| presenter_monitor.clone());
            let wa = new_window(audience.upcast_ref(), &title);
            let wp = new_window(presenter.upcast_ref(), &t!("{} – Presenter View", title));
            fullscreen_on(&wa, &audience_monitor);
            fullscreen_on(&wp, &presenter_monitor);
            (vec![wa, wp], None, Some((audience_monitor, presenter_monitor)))
        } else {
            let stack = gtk::Stack::new();
            stack.set_transition_type(gtk::StackTransitionType::Crossfade);
            stack.add_named(&audience, Some("audience"));
            stack.add_named(&presenter, Some("presenter"));
            stack.set_visible_child_name("audience");
            let win = new_window(stack.upcast_ref(), &title);
            if layout == Layout::Fullscreen {
                match &main_monitor {
                    Some(m) => win.fullscreen_on_monitor(m),
                    None => win.fullscreen(),
                }
            }
            (vec![win], Some(stack), None)
        };

        let this = Rc::new(Self {
            app: app.clone(),
            audience: audience.clone(),
            presenter: presenter.clone(),
            windows: RefCell::new(windows),
            stack,
            monitors: RefCell::new(pair),
            inhibit: Cell::new(0),
            ended: Cell::new(false),
            on_end: RefCell::new(Some(Box::new(on_end))),
        });

        this.connect_messages(&audience, View::Audience);
        this.connect_messages(&presenter, View::Presenter);
        for win in this.windows.borrow().iter() {
            let weak = Rc::downgrade(&this);
            win.connect_close_request(move |_| {
                if let Some(this) = weak.upgrade() {
                    this.end();
                }
                glib::Propagation::Proceed
            });
        }

        // Present the audience window first so the presenter window ends up
        // in front and gets the keyboard.
        let windows = this.windows.borrow().clone();
        for win in &windows {
            win.present();
        }
        let focus = if two_screens { presenter.clone() } else { audience.clone() };
        if let Some(win) = windows.last() {
            this.inhibit.set(app.inhibit(Some(win), gtk::ApplicationInhibitFlags::IDLE, Some(t!("Presentation running"))));
        }
        glib::idle_add_local_once(move || {
            focus.grab_focus();
        });
        this
    }

    fn connect_messages(self: &Rc<Self>, view: &webkit6::WebView, kind: View) {
        let Some(ucm) = view.user_content_manager() else { return };
        ucm.register_script_message_handler("qn", None);
        let weak: Weak<Self> = Rc::downgrade(self);
        ucm.connect_script_message_received(Some("qn"), move |_, value| {
            if let Some(this) = weak.upgrade() {
                this.on_message(kind, &value.to_str());
            }
        });
    }

    fn on_message(self: &Rc<Self>, from: View, json: &str) {
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(json) else { return };
        match msg.get("type").and_then(|t| t.as_str()).unwrap_or_default() {
            "exit" => self.end(),
            "ready" => {}
            "toggle-presenter" => self.toggle_presenter(),
            "swap" => self.swap_screens(),
            "fullscreen" => self.toggle_fullscreen(from),
            _ => {
                let other = match from {
                    View::Audience => &self.presenter,
                    View::Presenter => &self.audience,
                };
                let js = format!("QN.receive({json})");
                other.evaluate_javascript(&js, None, None, None::<&gio::Cancellable>, |_| {});
            }
        }
    }

    /// One window: switches between audience and presenter view.
    fn toggle_presenter(&self) {
        let Some(stack) = &self.stack else { return };
        let presenter = stack.visible_child_name().as_deref() != Some("presenter");
        stack.set_visible_child_name(if presenter { "presenter" } else { "audience" });
        let view = if presenter { &self.presenter } else { &self.audience };
        view.grab_focus();
    }

    /// Two screens: exchanges the monitors of audience and presenter view.
    fn swap_screens(&self) {
        let mut monitors = self.monitors.borrow_mut();
        let Some((a, p)) = monitors.take() else { return };
        let windows = self.windows.borrow();
        if let [wa, wp] = windows.as_slice() {
            fullscreen_on(wa, &p);
            fullscreen_on(wp, &a);
        }
        *monitors = Some((p, a));
    }

    fn toggle_fullscreen(&self, from: View) {
        let view = match from {
            View::Audience => &self.audience,
            View::Presenter => &self.presenter,
        };
        let Some(win) = view.root().and_downcast::<gtk::Window>() else { return };
        if win.is_fullscreen() {
            win.unfullscreen();
        } else {
            win.fullscreen();
        }
    }

    /// Closes all windows of the presentation.
    pub fn end(&self) {
        if self.ended.replace(true) {
            return;
        }
        if self.inhibit.get() != 0 {
            self.app.uninhibit(self.inhibit.replace(0));
        }
        let windows: Vec<gtk::Window> = self.windows.borrow_mut().drain(..).collect();
        for win in windows {
            win.close();
        }
        if let Some(f) = self.on_end.borrow_mut().take() {
            f();
        }
    }
}

/// Puts `win` fullscreen on `monitor`.
///
/// On X11 the window manager places a window that is not mapped yet on the
/// active screen and ignores the requested position, and it does not move
/// fullscreen windows. So the window is shown first, taken out of
/// fullscreen if needed, and then moved to the monitor and made fullscreen
/// (`fullscreen_on_monitor` moves a mapped window before going fullscreen).
fn fullscreen_on(win: &gtk::Window, monitor: &gdk::Monitor) {
    fn apply(win: &gtk::Window, monitor: &gdk::Monitor) {
        let delay = if win.is_fullscreen() {
            win.unfullscreen();
            250
        } else {
            50
        };
        let (win, monitor) = (win.clone(), monitor.clone());
        glib::timeout_add_local_once(std::time::Duration::from_millis(delay), move || {
            win.fullscreen_on_monitor(&monitor);
        });
    }
    if win.is_mapped() {
        apply(win, monitor);
        return;
    }
    let handler: Rc<RefCell<Option<glib::SignalHandlerId>>> = Rc::default();
    let slot = handler.clone();
    let monitor = monitor.clone();
    let id = win.connect_map(move |win| {
        if let Some(id) = slot.borrow_mut().take() {
            win.disconnect(id);
        }
        apply(win, &monitor);
    });
    *handler.borrow_mut() = Some(id);
}

fn web_view() -> webkit6::WebView {
    let ucm = webkit6::UserContentManager::new();
    let view = webkit6::WebView::builder().user_content_manager(&ucm).build();
    view.set_hexpand(true);
    view.set_vexpand(true);
    view.set_can_focus(true);
    view.set_background_color(&gdk::RGBA::BLACK);
    if let Some(s) = WebViewExt::settings(&view) {
        s.set_enable_developer_extras(false);
        s.set_javascript_can_open_windows_automatically(false);
        s.set_enable_back_forward_navigation_gestures(false);
    }
    // No context menu (right click goes back one step).
    view.connect_context_menu(|_, _, _| true);
    // Links open in the browser; the page itself never navigates away.
    let loaded = Rc::new(Cell::new(false));
    let flag = loaded.clone();
    view.connect_load_changed(move |_, event| {
        if event == webkit6::LoadEvent::Finished {
            flag.set(true);
        }
    });
    view.connect_decide_policy(move |_, decision, kind| {
        if kind != webkit6::PolicyDecisionType::NavigationAction {
            return false;
        }
        let Some(nav) = decision.downcast_ref::<webkit6::NavigationPolicyDecision>() else { return false };
        let Some(mut action) = nav.navigation_action() else { return false };
        if action.navigation_type() == webkit6::NavigationType::LinkClicked {
            if let Some(uri) = action.request().and_then(|r| r.uri()) {
                gtk::UriLauncher::new(&uri).launch(None::<&gtk::Window>, None::<&gio::Cancellable>, |_| {});
            }
            decision.ignore();
            return true;
        }
        if loaded.get() {
            decision.ignore();
            return true;
        }
        false
    });
    view
}
