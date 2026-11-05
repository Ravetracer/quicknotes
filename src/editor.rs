//! A text tab: GtkSourceView editor plus an optional live preview
//! (Markdown or PlantUML) in a split pane.

use crate::store::{TabKind, TabMeta};
use crate::{langs, markdown, plantuml};
use gtk::prelude::*;
use gtk::{gdk, gio, glib};
use sourceview5::prelude::*;
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;
use webkit6::prelude::*;

/// State shared by all tabs of the window.
pub struct Shared {
    pub dark: Cell<bool>,
    /// Preview zoom factor, follows the editor font size.
    pub zoom: Cell<f64>,
    pub plantuml_command: RefCell<Option<String>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PreviewKind {
    Markdown,
    PlantUml,
}

impl PreviewKind {
    fn for_language(id: Option<&str>) -> Option<Self> {
        match id {
            Some("markdown") => Some(Self::Markdown),
            Some("plantuml") => Some(Self::PlantUml),
            _ => None,
        }
    }
}

/// Where to put the cursor after (re)loading a file.
#[derive(Clone, Copy)]
pub enum CursorTarget {
    Start,
    End,
    Offset(i32),
}

struct Preview {
    webview: webkit6::WebView,
    loaded: Cell<bool>,
    pending: RefCell<Option<String>>,
}

pub type WidgetHook = Box<dyn Fn(&gtk::Widget)>;

pub struct EditorTab {
    pub id: String,
    pub root: gtk::Paned,
    pub view: sourceview5::View,
    pub buffer: sourceview5::Buffer,
    pub path: RefCell<Option<PathBuf>>,
    pub custom_title: RefCell<Option<String>>,
    pub language_locked: Cell<bool>,
    pub large: Cell<bool>,
    pub loading: Cell<bool>,
    shared: Rc<Shared>,
    preview: RefCell<Option<Preview>>,
    preview_on: Cell<bool>,
    needs_split: Cell<bool>,
    preview_timer: RefCell<Option<glib::SourceId>>,
    rendering: Cell<bool>,
    rerender: Cell<bool>,
    search: RefCell<Option<sourceview5::SearchContext>>,
    /// Called with the preview widget when it is created (used to add drop targets).
    pub webview_hook: RefCell<Option<WidgetHook>>,
}

impl EditorTab {
    pub fn new(id: String, shared: Rc<Shared>) -> Rc<Self> {
        let buffer = sourceview5::Buffer::new(None::<&gtk::TextTagTable>);
        buffer.set_highlight_matching_brackets(true);

        let view = sourceview5::View::with_buffer(&buffer);
        view.set_show_line_numbers(true);
        view.set_highlight_current_line(true);
        view.set_auto_indent(true);
        view.set_tab_width(4);
        view.set_insert_spaces_instead_of_tabs(true);
        view.set_smart_backspace(true);
        view.set_smart_home_end(sourceview5::SmartHomeEndType::Before);
        view.set_monospace(true);
        view.set_top_margin(6);
        view.set_bottom_margin(120);
        view.set_left_margin(6);
        view.set_right_margin(12);
        view.add_css_class("editor-view");

        let scrolled = gtk::ScrolledWindow::builder()
            .child(&view)
            .hexpand(true)
            .vexpand(true)
            .build();

        let root = gtk::Paned::new(gtk::Orientation::Horizontal);
        root.set_start_child(Some(&scrolled));
        root.set_shrink_start_child(false);
        root.set_shrink_end_child(false);
        root.set_widget_name(&id);

        let tab = Rc::new(Self {
            id,
            root,
            view,
            buffer,
            path: RefCell::new(None),
            custom_title: RefCell::new(None),
            language_locked: Cell::new(false),
            large: Cell::new(false),
            loading: Cell::new(false),
            shared,
            preview: RefCell::new(None),
            preview_on: Cell::new(false),
            needs_split: Cell::new(false),
            preview_timer: RefCell::new(None),
            rendering: Cell::new(false),
            rerender: Cell::new(false),
            search: RefCell::new(None),
            webview_hook: RefCell::new(None),
        });

        let weak = Rc::downgrade(&tab);
        tab.buffer.connect_changed(move |_| {
            if let Some(t) = weak.upgrade() {
                if !t.loading.get() {
                    let delay = match t.preview_kind() {
                        Some(PreviewKind::PlantUml) => 700,
                        _ => 250,
                    };
                    t.schedule_render(delay);
                }
            }
        });

        // Split the pane in half once the size is known.
        let weak = Rc::downgrade(&tab);
        tab.root.connect_notify_local(Some("max-position"), move |paned, _| {
            if let Some(t) = weak.upgrade() {
                let max = paned.max_position();
                if t.needs_split.get() && max > 100 && max < i32::MAX / 2 {
                    t.needs_split.set(false);
                    paned.set_position(paned.max_position() / 2);
                }
            }
        });

        tab.apply_theme();
        tab
    }

    // ----- content -------------------------------------------------------

    pub fn text(&self) -> String {
        let (s, e) = self.buffer.bounds();
        self.buffer.text(&s, &e, true).into()
    }

    pub fn set_text_irreversible(&self, text: &str) {
        let was_loading = self.loading.replace(true);
        self.buffer.begin_irreversible_action();
        self.buffer.set_text(text);
        self.buffer.end_irreversible_action();
        self.loading.set(was_loading);
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.char_count() == 0
    }

    /// Content must be kept in the notes store (notes, or files with unsaved changes).
    pub fn needs_note_file(&self) -> bool {
        self.path.borrow().is_none() || self.buffer.is_modified()
    }

    pub fn place_cursor(&self, target: CursorTarget) {
        let iter = match target {
            CursorTarget::Start => self.buffer.start_iter(),
            CursorTarget::End => self.buffer.end_iter(),
            CursorTarget::Offset(o) => self.buffer.iter_at_offset(o),
        };
        self.buffer.place_cursor(&iter);
        let view = self.view.clone();
        let buffer = self.buffer.clone();
        glib::idle_add_local_once(move || {
            view.scroll_to_mark(&buffer.get_insert(), 0.1, false, 0.0, 0.0);
        });
    }

    pub fn cursor_target(&self) -> CursorTarget {
        let iter = self.buffer.iter_at_mark(&self.buffer.get_insert());
        if iter.is_end() && iter.offset() > 0 {
            CursorTarget::End
        } else {
            CursorTarget::Offset(iter.offset())
        }
    }

    /// Loads a file asynchronously. Large files are inserted in chunks so the
    /// UI stays responsive.
    pub async fn load_file(self: Rc<Self>, path: PathBuf, cursor: CursorTarget) -> Result<(), String> {
        self.loading.set(true);
        let result = gio::File::for_path(&path).load_contents_future().await;
        let bytes = match result {
            Ok((bytes, _)) => bytes,
            Err(e) => {
                self.loading.set(false);
                return Err(e.message().to_owned());
            }
        };
        let text: Cow<str> = String::from_utf8_lossy(&bytes);

        const CHUNK: usize = 1 << 20;
        self.buffer.begin_irreversible_action();
        self.buffer.set_text("");
        let mut pos = 0;
        while pos < text.len() {
            let mut end = (pos + CHUNK).min(text.len());
            while !text.is_char_boundary(end) {
                end += 1;
            }
            let mut iter = self.buffer.end_iter();
            self.buffer.insert(&mut iter, &text[pos..end]);
            pos = end;
            if pos < text.len() {
                glib::timeout_future(Duration::from_millis(1)).await;
            }
        }
        self.buffer.end_irreversible_action();
        self.buffer.set_modified(false);
        self.loading.set(false);
        self.place_cursor(cursor);
        self.schedule_render(0);
        Ok(())
    }

    pub fn save_to(&self, path: &Path) -> Result<(), String> {
        std::fs::write(path, self.text()).map_err(|e| e.to_string())?;
        *self.path.borrow_mut() = Some(path.to_owned());
        self.buffer.set_modified(false);
        Ok(())
    }

    // ----- appearance ----------------------------------------------------

    pub fn title(&self) -> String {
        if let Some(t) = self.custom_title.borrow().as_ref() {
            return t.clone();
        }
        if let Some(name) = self.path.borrow().as_ref().and_then(|p| p.file_name()) {
            return name.to_string_lossy().into_owned();
        }
        let mut iter = self.buffer.start_iter();
        for _ in 0..50 {
            let mut end = iter;
            if !end.ends_line() {
                end.forward_to_line_end();
            }
            let line = self.buffer.text(&iter, &end, false);
            let line = line.trim();
            if line.chars().any(char::is_alphanumeric) {
                return truncate(line.trim_start_matches(['#', ' ']), 32);
            }
            // Lines like "{" or "<" make poor titles; name data snippets by language.
            if !line.is_empty() {
                if let Some(lang) = self.buffer.language() {
                    return format!("{} snippet", lang.name());
                }
            }
            if !iter.forward_line() {
                break;
            }
        }
        "Untitled".into()
    }

    pub fn set_large(&self, large: bool) {
        self.large.set(large);
        self.buffer.set_highlight_syntax(!large);
        self.buffer.set_highlight_matching_brackets(!large);
        if large {
            self.set_wrap(false);
        }
    }

    pub fn wrap(&self) -> bool {
        self.view.wrap_mode() != gtk::WrapMode::None
    }

    pub fn set_wrap(&self, wrap: bool) {
        self.view.set_wrap_mode(if wrap { gtk::WrapMode::WordChar } else { gtk::WrapMode::None });
    }

    pub fn language_id(&self) -> Option<String> {
        self.buffer.language().map(|l| l.id().to_string())
    }

    pub fn set_language(self: &Rc<Self>, lang: Option<&sourceview5::Language>) {
        self.buffer.set_language(lang);
        self.sync_preview();
    }

    pub fn set_language_id(self: &Rc<Self>, id: Option<&str>) {
        self.set_language(id.and_then(langs::by_id).as_ref());
    }

    pub fn apply_theme(&self) {
        let dark = self.shared.dark.get();
        let sm = sourceview5::StyleSchemeManager::default();
        let scheme = if dark {
            sm.scheme("Adwaita-dark").or_else(|| sm.scheme("oblivion"))
        } else {
            sm.scheme("Adwaita").or_else(|| sm.scheme("classic"))
        };
        self.buffer.set_style_scheme(scheme.as_ref());
        if let Some(p) = self.preview.borrow().as_ref() {
            p.webview.set_background_color(&preview_bg(dark));
            let js = format!("setTheme('{}')", if dark { "dark" } else { "light" });
            if p.loaded.get() {
                p.webview.evaluate_javascript(&js, None, None, None::<&gio::Cancellable>, |_| {});
            }
        }
    }

    pub fn apply_zoom(&self) {
        if let Some(p) = self.preview.borrow().as_ref() {
            p.webview.set_zoom_level(self.shared.zoom.get());
        }
    }

    // ----- search --------------------------------------------------------

    /// Returns the search context of this buffer and whether it was just created.
    pub fn search_context(&self, settings: &sourceview5::SearchSettings) -> (sourceview5::SearchContext, bool) {
        let mut slot = self.search.borrow_mut();
        match slot.as_ref() {
            Some(c) => (c.clone(), false),
            None => {
                let c = sourceview5::SearchContext::new(&self.buffer, Some(settings));
                *slot = Some(c.clone());
                (c, true)
            }
        }
    }

    pub fn set_search_highlight(&self, on: bool) {
        if let Some(c) = self.search.borrow().as_ref() {
            c.set_highlight(on);
        }
    }

    // ----- preview -------------------------------------------------------

    pub fn preview_kind(&self) -> Option<PreviewKind> {
        PreviewKind::for_language(self.language_id().as_deref())
    }

    pub fn preview_visible(&self) -> bool {
        self.preview_on.get() && self.preview_kind().is_some()
    }

    pub fn set_preview(self: &Rc<Self>, on: bool) {
        self.preview_on.set(on);
        self.sync_preview();
    }

    fn sync_preview(self: &Rc<Self>) {
        if self.preview_visible() {
            let webview = self.ensure_webview();
            if self.root.end_child().as_ref() != Some(webview.upcast_ref()) {
                self.root.set_end_child(Some(&webview));
                // Unallocated panes report max-position = G_MAXINT; use the real width.
                let width = self.root.width();
                if width > 100 {
                    self.root.set_position(width / 2);
                } else {
                    self.needs_split.set(true);
                }
            }
            self.schedule_render(0);
        } else if self.root.end_child().is_some() {
            self.root.set_end_child(None::<&gtk::Widget>);
        }
    }

    fn ensure_webview(self: &Rc<Self>) -> webkit6::WebView {
        if let Some(p) = self.preview.borrow().as_ref() {
            return p.webview.clone();
        }
        let dark = self.shared.dark.get();
        let webview = webkit6::WebView::new();
        webview.set_hexpand(true);
        webview.set_vexpand(true);
        webview.set_background_color(&preview_bg(dark));
        webview.set_zoom_level(self.shared.zoom.get());
        if let Some(s) = WebViewExt::settings(&webview) {
            s.set_enable_developer_extras(false);
            s.set_javascript_can_open_windows_automatically(false);
            s.set_enable_back_forward_navigation_gestures(false);
        }

        let weak = Rc::downgrade(self);
        webview.connect_load_changed(move |wv, event| {
            if event != webkit6::LoadEvent::Finished {
                return;
            }
            if let Some(t) = weak.upgrade() {
                if let Some(p) = t.preview.borrow().as_ref() {
                    p.loaded.set(true);
                    if let Some(js) = p.pending.borrow_mut().take() {
                        wv.evaluate_javascript(&js, None, None, None::<&gio::Cancellable>, |_| {});
                    }
                }
                t.nudge_split();
            }
        });

        // Repaint workaround, see `nudge_split` (covers tabs shown later).
        let weak = Rc::downgrade(self);
        webview.connect_map(move |_| {
            let weak = weak.clone();
            glib::timeout_add_local_once(Duration::from_millis(80), move || {
                if let Some(t) = weak.upgrade() {
                    t.nudge_split();
                }
            });
        });

        // Links open in the default browser; apart from loading the page shell
        // the preview never navigates (e.g. when a file is dropped onto it).
        let weak = Rc::downgrade(self);
        webview.connect_decide_policy(move |_, decision, kind| {
            if kind != webkit6::PolicyDecisionType::NavigationAction {
                return false;
            }
            let Some(nav) = decision.downcast_ref::<webkit6::NavigationPolicyDecision>() else {
                return false;
            };
            let Some(mut action) = nav.navigation_action() else {
                return false;
            };
            if action.navigation_type() == webkit6::NavigationType::LinkClicked {
                if let Some(uri) = action.request().and_then(|r| r.uri()) {
                    gtk::UriLauncher::new(&uri).launch(None::<&gtk::Window>, None::<&gio::Cancellable>, |_| {});
                }
                decision.ignore();
                return true;
            }
            let loaded = weak
                .upgrade()
                .and_then(|t| t.preview.try_borrow().ok().and_then(|p| p.as_ref().map(|p| p.loaded.get())))
                .unwrap_or(false);
            if loaded {
                decision.ignore();
                return true;
            }
            false
        });
        if let Some(hook) = self.webview_hook.borrow().as_ref() {
            hook(webview.upcast_ref());
        }

        let base = self
            .path
            .borrow()
            .as_ref()
            .and_then(|p| p.parent())
            .map(|d| format!("{}/", gio::File::for_path(d).uri()));
        webview.load_html(&markdown::page(dark), base.as_deref());

        *self.preview.borrow_mut() = Some(Preview {
            webview: webview.clone(),
            loaded: Cell::new(false),
            pending: RefCell::new(None),
        });
        webview
    }

    /// WebKitGTK does not paint a web view that was added to an already
    /// visible window until its size changes; wiggle the split by a pixel.
    fn nudge_split(&self) {
        if !self.root.is_mapped() || self.root.width() <= 0 || self.root.end_child().is_none() {
            return;
        }
        let root = self.root.clone();
        let pos = root.position();
        root.set_position(pos + 1);
        glib::timeout_add_local_once(Duration::from_millis(60), move || root.set_position(pos));
    }

    fn set_preview_html(&self, html: &str) {
        let Some(p) = self.preview.borrow().as_ref().map(|p| (p.webview.clone(), p.loaded.get())) else {
            return;
        };
        let js = format!("setContent({})", serde_json::to_string(html).unwrap_or_default());
        if p.1 {
            p.0.evaluate_javascript(&js, None, None, None::<&gio::Cancellable>, |_| {});
        } else if let Some(preview) = self.preview.borrow().as_ref() {
            *preview.pending.borrow_mut() = Some(js);
        }
    }

    pub fn schedule_render(self: &Rc<Self>, delay_ms: u64) {
        if !self.preview_visible() {
            return;
        }
        if let Some(id) = self.preview_timer.borrow_mut().take() {
            id.remove();
        }
        let weak = Rc::downgrade(self);
        let id = glib::timeout_add_local_once(Duration::from_millis(delay_ms), move || {
            if let Some(t) = weak.upgrade() {
                t.preview_timer.borrow_mut().take();
                t.render();
            }
        });
        *self.preview_timer.borrow_mut() = Some(id);
    }

    fn render(self: &Rc<Self>) {
        match self.preview_kind() {
            Some(PreviewKind::Markdown) => self.set_preview_html(&markdown::to_html(&self.text())),
            Some(PreviewKind::PlantUml) => self.render_plantuml(),
            None => {}
        }
    }

    fn render_plantuml(self: &Rc<Self>) {
        if self.rendering.get() {
            self.rerender.set(true);
            return;
        }
        let source = self.text();
        if source.trim().is_empty() {
            self.set_preview_html("<p class=\"message\">Start a diagram with @startuml … @enduml</p>");
            return;
        }
        let Some(cmd) = plantuml::command(self.shared.plantuml_command.borrow().as_deref()) else {
            self.set_preview_html(
                "<p class=\"message error\">PlantUML not found.\n\nInstall it (e.g. `apt install plantuml`), put a \
                 `plantuml` script on your PATH, or set \"plantuml_command\" in \
                 ~/.local/share/quicknotes/session.json.</p>",
            );
            return;
        };
        self.rendering.set(true);
        let weak = Rc::downgrade(self);
        glib::spawn_future_local(async move {
            let result = plantuml::render(cmd, source).await;
            let Some(t) = weak.upgrade() else { return };
            t.rendering.set(false);
            match result {
                Ok(svg) => t.set_preview_html(&format!("<div class=\"diagram\"><div class=\"card\">{svg}</div></div>")),
                Err(e) => t.set_preview_html(&format!("<p class=\"message error\">{}</p>", markdown::escape(&e))),
            }
            if t.rerender.replace(false) {
                t.schedule_render(0);
            }
        });
    }

    // ----- persistence ---------------------------------------------------

    pub fn meta(&self) -> TabMeta {
        let path = self.path.borrow().clone();
        TabMeta {
            id: self.id.clone(),
            kind: TabKind::Text,
            title: self.custom_title.borrow().clone(),
            modified: path.is_some() && self.buffer.is_modified(),
            path,
            language: self.language_id(),
            language_locked: self.language_locked.get(),
            wrap: self.wrap(),
            preview: self.preview_on.get(),
            cursor: self.buffer.cursor_position(),
        }
    }
}

fn preview_bg(dark: bool) -> gdk::RGBA {
    if dark {
        gdk::RGBA::new(0x1e as f32 / 255.0, 0x1e as f32 / 255.0, 0x1e as f32 / 255.0, 1.0)
    } else {
        gdk::RGBA::WHITE
    }
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_owned()
    } else {
        let mut t: String = s.chars().take(max - 1).collect();
        t.push('…');
        t
    }
}
