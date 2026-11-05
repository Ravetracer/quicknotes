//! Main window: tab management, actions, menus, search bar, status bar and
//! session persistence.

use crate::editor::{CursorTarget, EditorTab, PreviewKind, Shared};
use crate::image_tab::{self, ImageTab};
use crate::store::{self, Session, Settings, TabKind, TabMeta};
use crate::{format, html, langs, APP_ID};
use adw::prelude::*;
use gtk::{gdk, gio, glib};
use sourceview5::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};
use std::time::Duration;

const MIN_FONT: f64 = 6.0;
const MAX_FONT: f64 = 48.0;
const BASE_FONT: f64 = 12.0;

#[derive(Clone)]
pub enum Tab {
    Text(Rc<EditorTab>),
    Image(Rc<ImageTab>),
}

impl Tab {
    fn widget(&self) -> gtk::Widget {
        match self {
            Tab::Text(t) => t.root.clone().upcast(),
            Tab::Image(i) => i.root.clone().upcast(),
        }
    }
    fn title(&self) -> String {
        match self {
            Tab::Text(t) => t.title(),
            Tab::Image(i) => i.title(),
        }
    }
    fn path(&self) -> Option<PathBuf> {
        match self {
            Tab::Text(t) => t.path.borrow().clone(),
            Tab::Image(i) => Some(i.path.clone()),
        }
    }
    fn meta(&self) -> TabMeta {
        match self {
            Tab::Text(t) => t.meta(),
            Tab::Image(i) => i.meta(),
        }
    }
}

#[derive(Clone)]
struct Entry {
    tab: Tab,
    page: adw::TabPage,
}

struct SearchUi {
    revealer: gtk::Revealer,
    find: gtk::SearchEntry,
    replace: gtk::Entry,
    replace_row: gtk::Box,
    count: gtk::Label,
    settings: sourceview5::SearchSettings,
}

pub struct Window {
    win: adw::ApplicationWindow,
    view: adw::TabView,
    toasts: adw::ToastOverlay,
    stack: gtk::Stack,
    title: adw::WindowTitle,
    tabs: RefCell<HashMap<String, Entry>>,
    settings: RefCell<Settings>,
    shared: Rc<Shared>,
    editor_css: gtk::CssProvider,
    status_info: gtk::Label,
    status_pos: gtk::Label,
    zoom_btn: gtk::Button,
    font_btn: gtk::FontDialogButton,
    lang_drop: gtk::DropDown,
    lang_ids: Vec<Option<String>>,
    theme_btn: gtk::Button,
    guard: Cell<bool>,
    search: SearchUi,
    closed: RefCell<Vec<(TabMeta, Option<String>)>>,
    dirty: RefCell<HashSet<String>>,
    save_pending: Cell<bool>,
    scroll_acc: Cell<f64>,
    menu_page: RefCell<Option<adw::TabPage>>,
    /// Closing all tabs for a restore: no confirmations, no undo.
    bulk_close: Cell<bool>,
}

thread_local! {
    static WINDOW: RefCell<Option<Rc<Window>>> = const { RefCell::new(None) };
}

/// Shows the window (creating and restoring it on first call) and opens `files`.
pub fn present(app: &adw::Application, files: &[gio::File]) {
    let existing = WINDOW.with(|w| w.borrow().clone());
    let window = match existing {
        Some(w) => w,
        None => {
            let session = store::load_session();
            let w = Window::new(app, session.settings.clone());
            WINDOW.with(|c| *c.borrow_mut() = Some(w.clone()));
            w.restore(session);
            w
        }
    };
    for path in files.iter().filter_map(|f| f.path()) {
        window.open_path(path);
    }
    window.win.present();
}

/// Persists everything; used on SIGTERM/SIGINT.
pub fn persist_now() {
    if let Some(w) = WINDOW.with(|w| w.borrow().clone()) {
        w.persist_all();
    }
}

impl Window {
    fn new(app: &adw::Application, settings: Settings) -> Rc<Self> {
        adw::StyleManager::default().set_color_scheme(if settings.dark {
            adw::ColorScheme::ForceDark
        } else {
            adw::ColorScheme::ForceLight
        });

        let win = adw::ApplicationWindow::builder()
            .application(app)
            .title("QuickNotes")
            .icon_name(APP_ID)
            .default_width(settings.width.max(400))
            .default_height(settings.height.max(300))
            .build();
        if settings.maximized {
            win.maximize();
        }

        // ----- header bar ------------------------------------------------
        let header = adw::HeaderBar::new();
        let title = adw::WindowTitle::new("QuickNotes", "");
        header.set_title_widget(Some(&title));

        let icon_button = |icon: &str, tip: &str, action: &str| {
            gtk::Button::builder()
                .icon_name(icon)
                .tooltip_text(tip)
                .action_name(action)
                .build()
        };
        header.pack_start(&icon_button("tab-new-symbolic", "New note (Ctrl+N)", "win.new-note"));
        header.pack_start(&icon_button("document-open-symbolic", "Open file (Ctrl+O)", "win.open"));
        header.pack_start(&icon_button("document-save-symbolic", "Save to file (Ctrl+S)", "win.save"));

        let menu_btn = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .menu_model(&main_menu())
            .primary(true)
            .tooltip_text("Main menu")
            .build();
        let tools_btn = gtk::MenuButton::builder()
            .icon_name("format-justify-left-symbolic")
            .menu_model(&tools_menu())
            .tooltip_text("Format & tools")
            .build();
        let theme_btn = icon_button("weather-clear-symbolic", "Toggle dark/light theme", "win.dark");
        let preview_btn = gtk::ToggleButton::builder()
            .icon_name("view-dual-symbolic")
            .tooltip_text("Toggle preview (Ctrl+Shift+V)")
            .action_name("win.preview")
            .build();
        let find_btn = icon_button("edit-find-symbolic", "Find (Ctrl+F)", "win.find");
        header.pack_end(&menu_btn);
        header.pack_end(&theme_btn);
        header.pack_end(&tools_btn);
        header.pack_end(&preview_btn);
        header.pack_end(&find_btn);

        // ----- tabs ------------------------------------------------------
        let view = adw::TabView::new();
        let tab_menu = gio::Menu::new();
        tab_menu.append(Some("Rename…"), Some("win.tab-rename"));
        tab_menu.append(Some("Copy Path"), Some("win.tab-copy-path"));
        tab_menu.append(Some("Close Other Tabs"), Some("win.tab-close-others"));
        tab_menu.append(Some("Close"), Some("win.tab-close"));
        view.set_menu_model(Some(&tab_menu));
        let tab_bar = adw::TabBar::builder().view(&view).autohide(false).build();

        // ----- search bar ------------------------------------------------
        let search = build_search_ui();

        // ----- status bar ------------------------------------------------
        let status = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        status.add_css_class("statusbar");
        let status_info = gtk::Label::builder()
            .xalign(0.0)
            .hexpand(true)
            .ellipsize(gtk::pango::EllipsizeMode::Middle)
            .build();
        let status_pos = gtk::Label::new(None);
        let zoom_btn = gtk::Button::builder()
            .label("12 pt")
            .tooltip_text("Font size – Ctrl+wheel or Ctrl+/−, click to reset")
            .action_name("win.zoom-reset")
            .build();
        zoom_btn.add_css_class("flat");

        // Editor font family (the size is handled by the zoom above).
        let font_dialog = gtk::FontDialog::builder().title("Editor Font").modal(true).build();
        let font_btn = gtk::FontDialogButton::builder()
            .dialog(&font_dialog)
            .level(gtk::FontLevel::Family)
            .use_font(true)
            .use_size(false)
            .tooltip_text("Editor font")
            .build();
        font_btn.add_css_class("flat");
        font_btn.set_font_desc(&gtk::pango::FontDescription::from_string(&settings.font_family));

        let languages = langs::all();
        let mut lang_ids: Vec<Option<String>> = vec![None];
        let mut names: Vec<String> = vec!["Plain Text".into()];
        for (id, name) in languages {
            lang_ids.push(Some(id));
            names.push(name);
        }
        let name_refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let lang_drop = gtk::DropDown::from_strings(&name_refs);
        lang_drop.set_enable_search(true);
        lang_drop.set_search_match_mode(gtk::StringFilterMatchMode::Substring);
        lang_drop.set_expression(Some(gtk::PropertyExpression::new(
            gtk::StringObject::static_type(),
            None::<&gtk::Expression>,
            "string",
        )));
        lang_drop.set_tooltip_text(Some("Syntax"));
        lang_drop.add_css_class("flat");

        status.append(&status_info);
        status.append(&status_pos);
        status.append(&font_btn);
        status.append(&zoom_btn);
        status.append(&lang_drop);

        // ----- layout ----------------------------------------------------
        let empty = adw::StatusPage::builder()
            .icon_name(APP_ID)
            .title("No Open Notes")
            .description("Create a note with Ctrl+N or open a file with Ctrl+O.\nFiles can also be dropped here.")
            .build();
        let new_btn = gtk::Button::builder()
            .label("New Note")
            .action_name("win.new-note")
            .halign(gtk::Align::Center)
            .build();
        new_btn.add_css_class("pill");
        new_btn.add_css_class("suggested-action");
        empty.set_child(Some(&new_btn));

        let stack = gtk::Stack::new();
        stack.add_named(&view, Some("tabs"));
        stack.add_named(&empty, Some("empty"));

        let toasts = adw::ToastOverlay::new();
        toasts.set_child(Some(&stack));

        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.add_top_bar(&tab_bar);
        toolbar.add_top_bar(&search.revealer);
        toolbar.add_bottom_bar(&status);
        toolbar.set_content(Some(&toasts));
        win.set_content(Some(&toolbar));

        let editor_css = gtk::CssProvider::new();
        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().expect("no display"),
            &editor_css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let shared = Rc::new(Shared {
            dark: Cell::new(settings.dark),
            zoom: Cell::new(settings.font_size / BASE_FONT),
            plantuml_command: RefCell::new(settings.plantuml_command.clone()),
        });

        let this = Rc::new(Self {
            win,
            view,
            toasts,
            stack,
            title,
            tabs: RefCell::new(HashMap::new()),
            settings: RefCell::new(settings),
            shared,
            editor_css,
            status_info,
            status_pos,
            zoom_btn,
            font_btn,
            lang_drop,
            lang_ids,
            theme_btn,
            guard: Cell::new(false),
            search,
            closed: RefCell::new(Vec::new()),
            dirty: RefCell::new(HashSet::new()),
            save_pending: Cell::new(false),
            scroll_acc: Cell::new(0.0),
            menu_page: RefCell::new(None),
            bulk_close: Cell::new(false),
        });

        this.setup_actions(app);
        this.setup_signals(&toolbar);
        this.setup_search();
        this.apply_font();
        this.apply_theme_ui();
        this
    }

    fn weak(self: &Rc<Self>) -> Weak<Self> {
        Rc::downgrade(self)
    }

    // ===== actions ========================================================

    fn action(self: &Rc<Self>, name: &str, accels: &[&str], f: impl Fn(&Rc<Self>) + 'static) {
        let action = gio::SimpleAction::new(name, None);
        let weak = self.weak();
        action.connect_activate(move |_, _| {
            if let Some(this) = weak.upgrade() {
                f(&this);
            }
        });
        self.win.add_action(&action);
        if !accels.is_empty() {
            if let Some(app) = self.win.application() {
                app.set_accels_for_action(&format!("win.{name}"), accels);
            }
        }
    }

    fn toggle_action(self: &Rc<Self>, name: &str, accels: &[&str], f: impl Fn(&Rc<Self>, bool) + 'static) {
        let action = gio::SimpleAction::new_stateful(name, None, &false.to_variant());
        let weak = self.weak();
        action.connect_activate(move |a, _| {
            let new = !a.state().and_then(|s| s.get::<bool>()).unwrap_or(false);
            a.set_state(&new.to_variant());
            if let Some(this) = weak.upgrade() {
                f(&this, new);
            }
        });
        self.win.add_action(&action);
        if let Some(app) = self.win.application() {
            app.set_accels_for_action(&format!("win.{name}"), accels);
        }
    }

    fn set_enabled(&self, name: &str, enabled: bool) {
        if let Some(a) = self.win.lookup_action(name).and_downcast::<gio::SimpleAction>() {
            a.set_enabled(enabled);
        }
    }

    fn set_toggle(&self, name: &str, enabled: bool, state: bool) {
        if let Some(a) = self.win.lookup_action(name).and_downcast::<gio::SimpleAction>() {
            a.set_enabled(enabled);
            a.set_state(&state.to_variant());
        }
    }

    fn setup_actions(self: &Rc<Self>, app: &adw::Application) {
        self.action("new-note", &["<Control>n", "<Control>t"], |w| {
            w.new_note();
        });
        self.action("open", &["<Control>o"], |w| w.open_dialog());
        self.action("save", &["<Control>s"], |w| w.save());
        self.action("save-as", &["<Control><Shift>s"], |w| w.save_as());
        self.action("reload", &["F5"], |w| w.reload());
        self.action("close-tab", &["<Control>w"], |w| {
            if let Some(p) = w.view.selected_page() {
                w.view.close_page(&p);
            }
        });
        self.action("reopen-closed", &["<Control><Shift>t"], |w| w.reopen_closed());
        self.action("rename", &["F2"], |w| {
            if let Some(p) = w.view.selected_page() {
                w.rename(&p);
            }
        });
        self.action("tab-rename", &[], |w| {
            if let Some(p) = w.menu_page() {
                w.rename(&p);
            }
        });
        self.action("tab-copy-path", &[], |w| {
            let path = w.menu_page().and_then(|p| w.entry_for_page(&p)).and_then(|e| e.tab.path());
            match path {
                Some(p) => {
                    w.win.clipboard().set_text(&p.to_string_lossy());
                    w.toast("Path copied");
                }
                None => w.toast("This note has no file"),
            }
        });
        self.action("tab-close", &[], |w| {
            if let Some(p) = w.menu_page() {
                w.view.close_page(&p);
            }
        });
        self.action("tab-close-others", &[], |w| {
            if let Some(p) = w.menu_page() {
                w.view.close_other_pages(&p);
            }
        });
        self.action("find", &["<Control>f"], |w| w.open_search(false));
        self.action("replace", &["<Control>h"], |w| w.open_search(true));
        self.action("find-next", &["F3"], |w| w.find_step(true, false));
        self.action("find-prev", &["<Shift>F3"], |w| w.find_step(false, false));
        self.action("zoom-in", &["<Control>plus", "<Control>equal", "<Control>KP_Add"], |w| w.zoom(1.0));
        self.action("zoom-out", &["<Control>minus", "<Control>KP_Subtract"], |w| w.zoom(-1.0));
        self.action("zoom-reset", &["<Control>0", "<Control>KP_0"], |w| {
            w.settings.borrow_mut().font_size = BASE_FONT;
            w.apply_font();
            w.schedule_persist();
        });
        self.action("export-html", &["<Control><Shift>e"], |w| w.export(ExportFormat::Html));
        self.action("export-pdf", &["<Control><Shift>p"], |w| w.export(ExportFormat::Pdf));
        self.action("backup", &[], |w| w.backup());
        self.action("restore-backup", &[], |w| w.restore_backup());
        self.action("open-data-dir", &[], |w| {
            let launcher = gtk::FileLauncher::new(Some(&gio::File::for_path(store::data_dir())));
            launcher.launch(Some(&w.win), None::<&gio::Cancellable>, |_| {});
        });
        self.action("about", &[], |w| {
            let about = adw::AboutDialog::builder()
                .application_name("QuickNotes")
                .application_icon(APP_ID)
                .version(env!("CARGO_PKG_VERSION"))
                .comments("A small tabbed notepad for text and code snippets.\nNotes are stored in ~/.local/share/quicknotes.")
                .license_type(gtk::License::MitX11)
                .build();
            about.present(Some(&w.win));
        });

        self.toggle_action("preview", &["<Control><Shift>v"], |w, on| {
            if let Some(t) = w.current_text() {
                t.set_preview(on);
                w.mark_dirty(&t.id);
            }
        });
        self.toggle_action("wrap", &["<Alt>z"], |w, on| {
            if let Some(t) = w.current_text() {
                t.set_wrap(on);
                w.mark_dirty(&t.id);
            }
        });
        self.toggle_action("dark", &[], |w, on| w.set_dark(on));
        self.set_toggle("dark", true, self.settings.borrow().dark);
        self.toggle_action("ligatures", &[], |w, on| {
            w.settings.borrow_mut().ligatures = on;
            w.apply_font();
            w.schedule_persist();
        });
        self.set_toggle("ligatures", true, self.settings.borrow().ligatures);

        let tool = gio::SimpleAction::new("tool", Some(glib::VariantTy::STRING));
        let weak = self.weak();
        tool.connect_activate(move |_, param| {
            if let (Some(this), Some(name)) = (weak.upgrade(), param.and_then(|p| p.str())) {
                this.apply_tool(name);
            }
        });
        self.win.add_action(&tool);
        app.set_accels_for_action("win.tool::beautify", &["<Control><Alt>b"]);
        app.set_accels_for_action("win.tool::minify", &["<Control><Alt>m"]);

        let quit = gio::SimpleAction::new("quit", None);
        let weak = self.weak();
        quit.connect_activate(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.win.close();
            }
        });
        app.add_action(&quit);
        app.set_accels_for_action("app.quit", &["<Control>q"]);
    }

    fn setup_signals(self: &Rc<Self>, toolbar: &adw::ToolbarView) {
        let weak = self.weak();
        self.font_btn.connect_font_desc_notify(move |btn| {
            let Some(this) = weak.upgrade() else { return };
            let Some(family) = btn.font_desc().and_then(|d| d.family()) else { return };
            if this.settings.borrow().font_family == family.as_str() {
                return;
            }
            this.settings.borrow_mut().font_family = family.to_string();
            this.apply_font();
            this.schedule_persist();
            // Back to the text, so typing continues right away.
            if let Some(t) = this.current_text() {
                t.view.grab_focus();
            }
        });

        let weak = self.weak();
        self.view.connect_selected_page_notify(move |_| {
            if let Some(this) = weak.upgrade() {
                this.sync_ui();
                if let Some(Tab::Text(t)) = this.current() {
                    if !this.search.revealer.reveals_child() {
                        t.view.grab_focus();
                    }
                }
            }
        });

        let weak = self.weak();
        self.view.connect_close_page(move |view, page| {
            let Some(this) = weak.upgrade() else {
                return glib::Propagation::Proceed;
            };
            if this.bulk_close.get() {
                if let Some(entry) = this.entry_for_page(page) {
                    let id = entry.page.child().widget_name().to_string();
                    this.tabs.borrow_mut().remove(&id);
                    this.dirty.borrow_mut().remove(&id);
                    store::delete_note(&id);
                }
                return glib::Propagation::Proceed;
            }
            match this.entry_for_page(page).map(|e| e.tab) {
                Some(Tab::Text(t)) if t.path.borrow().is_some() && t.buffer.is_modified() => {
                    this.confirm_close(view.clone(), page.clone(), t);
                    return glib::Propagation::Stop;
                }
                Some(tab) => this.forget(&tab),
                None => {}
            }
            glib::Propagation::Proceed
        });

        for signal in ["page-attached", "page-detached"] {
            let weak = self.weak();
            self.view.connect_local(signal, false, move |_| {
                if let Some(this) = weak.upgrade() {
                    this.sync_ui();
                }
                None
            });
        }

        let weak = self.weak();
        self.view.connect_setup_menu(move |_, page| {
            let Some(this) = weak.upgrade() else { return };
            match page {
                Some(p) => *this.menu_page.borrow_mut() = Some(p.clone()),
                None => {
                    // Clear after the menu action had a chance to run.
                    let weak = this.weak();
                    glib::idle_add_local_once(move || {
                        if let Some(this) = weak.upgrade() {
                            this.menu_page.borrow_mut().take();
                        }
                    });
                }
            }
        });

        let weak = self.weak();
        self.lang_drop.connect_selected_notify(move |d| {
            let Some(this) = weak.upgrade() else { return };
            if this.guard.get() {
                return;
            }
            let Some(t) = this.current_text() else { return };
            let id = this.lang_ids.get(d.selected() as usize).cloned().flatten();
            t.language_locked.set(true);
            t.set_language_id(id.as_deref());
            t.set_preview(t.preview_kind().is_some() && !t.large.get());
            this.mark_dirty(&t.id);
            this.sync_ui();
        });

        let weak = self.weak();
        self.win.connect_close_request(move |_| {
            if let Some(this) = weak.upgrade() {
                this.persist_all();
            }
            WINDOW.with(|w| w.borrow_mut().take());
            glib::Propagation::Proceed
        });

        // Drop files anywhere in the window to open them. Editor and preview
        // get their own targets (see `connect_text_tab`) because they handle
        // drops themselves. Bubble phase on purpose: capture-phase drop targets
        // on ancestors keep stale drop state in GTK 4.14 and break later drags.
        self.install_drop_target(toolbar.upcast_ref(), DropKind::Files, &DropSlot::default());
    }

    /// Installs a drop target on `widget`, replacing the one in `slot`.
    ///
    /// GTK 4.14 drop targets on X11 can keep a stale drop after unusual drag
    /// sequences (a tab switch while dragging, Chrome sending positions after
    /// the drop) and then silently ignore every later drop. Our targets are
    /// therefore replaced after each drop and whenever an editor is shown again.
    ///
    /// Dropped data is read explicitly instead of via GDK's conversions:
    /// - Qt apps (Dolphin) advertise `application/vnd.portal.filetransfer`,
    ///   which GDK prefers for file lists and which never resolves outside a
    ///   sandbox.
    /// - On X11, GTK 4 cannot read drag data from Qt sources at all, see
    ///   [`crate::xdnd`]; the data is fetched directly there first.
    fn install_drop_target(self: &Rc<Self>, widget: &gtk::Widget, kind: DropKind, slot: &DropSlot) {
        if let Some(old) = slot.borrow_mut().take() {
            widget.remove_controller(&old);
        }
        let mimes: Vec<&str> = match kind {
            DropKind::Files => vec![URI_LIST],
            DropKind::Editor(_) => std::iter::once(URI_LIST).chain(TEXT_MIMES.iter().copied()).collect(),
        };
        // COPY only: accepting MOVE could make the source delete the files.
        let target = gtk::DropTargetAsync::new(Some(gdk::ContentFormats::new(&mimes)), gdk::DragAction::COPY);
        let is_editor = matches!(kind, DropKind::Editor(_));
        // Drags started inside QuickNotes (moving text within a note) are
        // left to GtkTextView's own handling.
        let accept = move |drop: &gdk::Drop| {
            if is_editor && drop.drag().is_some() {
                gdk::DragAction::empty()
            } else {
                gdk::DragAction::COPY
            }
        };
        target.connect_drag_enter(move |_, drop, _, _| accept(drop));
        target.connect_drag_motion(move |_, drop, _, _| accept(drop));

        let weak = self.weak();
        let widget_ref = widget.downgrade();
        let slot_ref = Rc::downgrade(slot);
        target.connect_drop(move |_, drop, x, y| {
            if accept(drop).is_empty() {
                return false;
            }
            let Some(this) = weak.upgrade() else { return false };
            // Remember the insertion point now; reading the data is async.
            let insert_at = match &kind {
                DropKind::Editor(t) => t.upgrade().map(|t| {
                    let (bx, by) = t.view.window_to_buffer_coords(gtk::TextWindowType::Widget, x as i32, y as i32);
                    // Outside of text (e.g. below the last line): end of the nearest line.
                    let iter = t.view.iter_at_location(bx, by).unwrap_or_else(|| {
                        let (mut iter, _) = t.view.line_at_y(by);
                        if !iter.ends_line() {
                            iter.forward_to_line_end();
                        }
                        iter
                    });
                    let mark = t.buffer.create_mark(None, &iter, true);
                    (t, mark)
                }),
                DropKind::Files => None,
            };
            let drop = drop.clone();
            let kind = kind.clone();
            let widget_ref = widget_ref.clone();
            let slot_ref = slot_ref.clone();
            glib::spawn_future_local(async move {
                let ok = this.handle_drop(&drop, insert_at).await;
                drop.finish(if ok { gdk::DragAction::COPY } else { gdk::DragAction::empty() });
                if let (Some(widget), Some(slot)) = (widget_ref.upgrade(), slot_ref.upgrade()) {
                    this.install_drop_target(&widget, kind, &slot);
                }
            });
            true
        });
        widget.add_controller(target.clone());
        *slot.borrow_mut() = Some(target);
    }

    /// Opens dropped files, or inserts dropped text into the editor at `insert_at`.
    async fn handle_drop(self: &Rc<Self>, drop: &gdk::Drop, insert_at: Option<(Rc<EditorTab>, gtk::TextMark)>) -> bool {
        let xid = self.x11_window_id();
        let formats = drop.formats();
        let mut uri_text = None;
        if formats.contain_mime_type(URI_LIST) {
            if let Some(list) = read_drop_data(drop, xid, URI_LIST).await {
                let paths = parse_uri_list(&list);
                if !paths.is_empty() {
                    if let Some((t, mark)) = &insert_at {
                        t.buffer.delete_mark(mark);
                    }
                    for path in paths {
                        self.open_path(path);
                    }
                    return true;
                }
                uri_text = Some(uri_list_as_text(&list));
            }
        }
        let Some((t, mark)) = insert_at else {
            self.toast("Could not read the dropped files");
            return false;
        };
        let mut text = None;
        for mime in TEXT_MIMES.iter().filter(|m| formats.contain_mime_type(m)) {
            text = read_drop_data(drop, xid, mime).await;
            if text.is_some() {
                break;
            }
        }
        let text = text.or(uri_text).filter(|s| !s.is_empty());
        if let Some(text) = &text {
            let buffer = &t.buffer;
            let mut iter = buffer.iter_at_mark(&mark);
            buffer.begin_user_action();
            buffer.insert(&mut iter, text);
            buffer.end_user_action();
            buffer.place_cursor(&iter);
            t.view.grab_focus();
        } else {
            self.toast("Could not read the dropped text");
        }
        t.buffer.delete_mark(&mark);
        text.is_some()
    }

    /// XID of the toplevel window when running on X11.
    fn x11_window_id(&self) -> Option<u32> {
        let surface = self.win.surface()?;
        let x11 = surface.downcast_ref::<gdk4_x11::X11Surface>()?;
        u32::try_from(x11.xid()).ok()
    }

    // ===== tab bookkeeping ================================================

    fn menu_page(&self) -> Option<adw::TabPage> {
        self.menu_page.borrow().clone().or_else(|| self.view.selected_page())
    }

    fn entry_for_page(&self, page: &adw::TabPage) -> Option<Entry> {
        self.tabs.borrow().get(page.child().widget_name().as_str()).cloned()
    }

    fn current(&self) -> Option<Tab> {
        self.view
            .selected_page()
            .and_then(|p| self.entry_for_page(&p))
            .map(|e| e.tab)
    }

    fn current_text(&self) -> Option<Rc<EditorTab>> {
        match self.current() {
            Some(Tab::Text(t)) => Some(t),
            _ => None,
        }
    }

    fn is_current(&self, id: &str) -> bool {
        self.view
            .selected_page()
            .is_some_and(|p| p.child().widget_name() == id)
    }

    fn text_tabs(&self) -> Vec<Rc<EditorTab>> {
        self.tabs
            .borrow()
            .values()
            .filter_map(|e| match &e.tab {
                Tab::Text(t) => Some(t.clone()),
                _ => None,
            })
            .collect()
    }

    fn add_tab(self: &Rc<Self>, tab: Tab, select: bool) -> adw::TabPage {
        let widget = tab.widget();
        let id = widget.widget_name().to_string();
        let page = self.view.append(&widget);
        self.tabs.borrow_mut().insert(id.clone(), Entry { tab: tab.clone(), page: page.clone() });

        match &tab {
            Tab::Text(t) => self.connect_text_tab(t),
            Tab::Image(_) => page.set_icon(Some(&gio::ThemedIcon::new("image-x-generic-symbolic"))),
        }
        self.refresh_title(&id);
        if select {
            self.view.set_selected_page(&page);
        }
        page
    }

    fn connect_text_tab(self: &Rc<Self>, t: &Rc<EditorTab>) {
        let id = t.id.clone();
        let weak = self.weak();
        let tab = Rc::downgrade(t);
        t.buffer.connect_changed(move |_| {
            let (Some(this), Some(t)) = (weak.upgrade(), tab.upgrade()) else { return };
            if t.loading.get() {
                return;
            }
            this.mark_dirty(&t.id);
            this.refresh_title(&t.id);
            if this.is_current(&t.id) {
                this.update_status();
            }
        });

        let weak = self.weak();
        let tab_id = id.clone();
        t.buffer.connect_modified_changed(move |_| {
            if let Some(this) = weak.upgrade() {
                this.refresh_title(&tab_id);
                this.mark_dirty(&tab_id);
            }
        });

        for prop in ["cursor-position", "has-selection"] {
            let weak = self.weak();
            let tab_id = id.clone();
            t.buffer.connect_notify_local(Some(prop), move |_, _| {
                if let Some(this) = weak.upgrade() {
                    if this.is_current(&tab_id) {
                        this.update_position();
                    }
                }
            });
        }

        // Ctrl+mouse wheel changes the font size.
        let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        scroll.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = self.weak();
        scroll.connect_scroll(move |c, _, dy| {
            if !c.current_event_state().contains(gdk::ModifierType::CONTROL_MASK) {
                return glib::Propagation::Proceed;
            }
            if let Some(this) = weak.upgrade() {
                let acc = this.scroll_acc.get() + dy;
                let steps = acc.trunc();
                this.scroll_acc.set(acc - steps);
                if steps != 0.0 {
                    this.zoom(-steps);
                }
            }
            glib::Propagation::Stop
        });
        t.root.add_controller(scroll);

        // Drop targets for the editor (files and text) and the preview (files).
        // Re-installed whenever the editor is shown, see `install_drop_target`.
        let slot = DropSlot::default();
        self.install_drop_target(t.view.upcast_ref(), DropKind::Editor(Rc::downgrade(t)), &slot);
        let weak = self.weak();
        let tab = Rc::downgrade(t);
        t.view.connect_map(move |view| {
            if let Some(this) = weak.upgrade() {
                this.install_drop_target(view.upcast_ref(), DropKind::Editor(tab.clone()), &slot);
            }
        });
        let weak = self.weak();
        *t.webview_hook.borrow_mut() = Some(Box::new(move |webview: &gtk::Widget| {
            if let Some(this) = weak.upgrade() {
                this.install_drop_target(webview, DropKind::Files, &DropSlot::default());
            }
        }));
    }

    fn refresh_title(&self, id: &str) {
        let Some(entry) = self.tabs.borrow().get(id).cloned() else { return };
        let mut title = entry.tab.title();
        if let Tab::Text(t) = &entry.tab {
            if t.path.borrow().is_some() && t.buffer.is_modified() {
                title.push_str(" ●");
            }
        }
        if entry.page.title() != title {
            entry.page.set_title(&title);
        }
        let tooltip = entry
            .tab
            .path()
            .map(|p| markup_escape(&p.to_string_lossy()))
            .unwrap_or_else(|| "Note – stored automatically".into());
        entry.page.set_tooltip(&tooltip);
        if self.is_current(id) {
            self.update_window_title();
        }
    }

    fn new_note(self: &Rc<Self>) -> Rc<EditorTab> {
        let t = EditorTab::new(store::new_id(), self.shared.clone());
        t.set_wrap(true);
        self.add_tab(Tab::Text(t.clone()), true);
        t.view.grab_focus();
        self.mark_dirty(&t.id);
        t
    }

    /// Removes a tab from bookkeeping and remembers it for "Reopen Closed Tab".
    fn forget(self: &Rc<Self>, tab: &Tab) {
        let meta = tab.meta();
        let id = meta.id.clone();
        self.tabs.borrow_mut().remove(&id);
        self.dirty.borrow_mut().remove(&id);
        store::delete_note(&id);

        match tab {
            Tab::Text(t) => {
                let content = t.needs_note_file().then(|| t.text());
                let is_note = t.path.borrow().is_none();
                let has_content = content.as_deref().is_some_and(|c| !c.trim().is_empty());
                if has_content || !is_note {
                    self.closed.borrow_mut().push((meta, content));
                }
                if is_note && has_content {
                    let toast = adw::Toast::builder()
                        .title(format!("“{}” closed", t.title()))
                        .button_label("Undo")
                        .action_name("win.reopen-closed")
                        .timeout(6)
                        .build();
                    self.toasts.add_toast(toast);
                }
            }
            Tab::Image(_) => self.closed.borrow_mut().push((meta, None)),
        }
        self.schedule_persist();
    }

    fn confirm_close(self: &Rc<Self>, view: adw::TabView, page: adw::TabPage, t: Rc<EditorTab>) {
        let dialog = adw::AlertDialog::new(
            Some(&format!("Save changes to “{}”?", t.title())),
            Some("The file has unsaved changes. They will be lost if you don't save them."),
        );
        dialog.add_responses(&[("cancel", "_Cancel"), ("discard", "_Discard"), ("save", "_Save")]);
        dialog.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
        dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("save"));
        dialog.set_close_response("cancel");
        let this = self.clone();
        glib::spawn_future_local(async move {
            let response = dialog.choose_future(&this.win).await;
            let close = match response.as_str() {
                "save" => {
                    let path = t.path.borrow().clone();
                    match path.map(|p| t.save_to(&p)) {
                        Some(Ok(())) => true,
                        Some(Err(e)) => {
                            this.toast(&format!("Save failed: {e}"));
                            false
                        }
                        None => false,
                    }
                }
                "discard" => true,
                _ => false,
            };
            if close {
                this.forget(&Tab::Text(t));
            }
            view.close_page_finish(&page, close);
        });
    }

    fn reopen_closed(self: &Rc<Self>) {
        let item = self.closed.borrow_mut().pop();
        match item {
            Some((meta, content)) => {
                if let Some(page) = self.restore_tab(meta, content) {
                    self.view.set_selected_page(&page);
                }
            }
            None => self.toast("No closed tabs"),
        }
    }

    // ===== session ========================================================

    fn restore(self: &Rc<Self>, session: Session) {
        for meta in session.tabs {
            let content = store::read_note(&meta.id);
            self.restore_tab(meta, content);
        }
        if self.view.n_pages() == 0 {
            self.new_note();
        } else {
            let idx = (session.active as i32).min(self.view.n_pages() - 1);
            self.view.set_selected_page(&self.view.nth_page(idx));
        }
        self.sync_ui();
    }

    /// Recreates a tab from metadata. `content` overrides the file content.
    fn restore_tab(self: &Rc<Self>, meta: TabMeta, content: Option<String>) -> Option<adw::TabPage> {
        if self.tabs.borrow().contains_key(&meta.id) {
            return None;
        }
        if meta.kind == TabKind::Image {
            let path = meta.path?;
            return match ImageTab::new(meta.id, path.clone()) {
                Ok(i) => Some(self.add_tab(Tab::Image(i), false)),
                Err(e) => {
                    self.toast(&format!("Cannot open {}: {e}", path.display()));
                    None
                }
            };
        }

        if let (None, Some(p)) = (&content, &meta.path) {
            if !p.is_file() {
                self.toast(&format!("File not found: {}", p.display()));
                return None;
            }
        }

        let t = EditorTab::new(meta.id.clone(), self.shared.clone());
        *t.path.borrow_mut() = meta.path.clone();
        *t.custom_title.borrow_mut() = meta.title.clone();
        t.language_locked.set(meta.language_locked);
        t.set_wrap(meta.wrap);
        t.set_language_id(meta.language.as_deref());
        if let Some(p) = &meta.path {
            t.set_large(self.is_large(p));
        }
        let page = self.add_tab(Tab::Text(t.clone()), false);

        match (content, meta.path) {
            (Some(c), path) => {
                t.set_text_irreversible(&c);
                t.buffer.set_modified(path.is_some() && meta.modified);
                t.place_cursor(CursorTarget::Offset(meta.cursor));
                self.refresh_title(&t.id);
            }
            (None, Some(path)) => self.load_into(&t, path, CursorTarget::Offset(meta.cursor)),
            (None, None) => {}
        }
        t.set_preview(meta.preview);
        Some(page)
    }

    fn mark_dirty(self: &Rc<Self>, id: &str) {
        self.dirty.borrow_mut().insert(id.to_owned());
        self.schedule_persist();
    }

    fn schedule_persist(self: &Rc<Self>) {
        if self.save_pending.replace(true) {
            return;
        }
        let weak = self.weak();
        glib::timeout_add_local_once(Duration::from_millis(800), move || {
            if let Some(this) = weak.upgrade() {
                this.save_pending.set(false);
                this.persist();
            }
        });
    }

    fn persist_all(self: &Rc<Self>) {
        let ids: Vec<String> = self.tabs.borrow().keys().cloned().collect();
        self.dirty.borrow_mut().extend(ids);
        self.persist();
    }

    fn persist(self: &Rc<Self>) {
        let ids: Vec<String> = self.dirty.borrow_mut().drain().collect();
        for id in ids {
            let Some(Tab::Text(t)) = self.tabs.borrow().get(&id).map(|e| e.tab.clone()) else {
                continue;
            };
            if t.loading.get() {
                continue;
            }
            if t.needs_note_file() {
                let text = t.text();
                if let Err(e) = store::write_note(&id, &text) {
                    eprintln!("quicknotes: cannot store note {id}: {e}");
                    self.toast(&format!("Cannot store note: {e}"));
                }
                self.auto_detect_language(&t, &text);
            } else {
                store::delete_note(&id);
            }
        }
        if let Err(e) = store::save_session(&self.snapshot()) {
            eprintln!("quicknotes: cannot save session: {e}");
        }
    }

    /// Notes without a file get their syntax from the content until the
    /// user picks one explicitly.
    fn auto_detect_language(self: &Rc<Self>, t: &Rc<EditorTab>, text: &str) {
        if t.path.borrow().is_some() || t.language_locked.get() || text.len() > 512 * 1024 {
            return;
        }
        let detected = langs::sniff(text);
        if detected.map(str::to_owned) == t.language_id() {
            return;
        }
        t.set_language_id(detected);
        if detected == Some("plantuml") {
            t.set_preview(true);
        }
        if self.is_current(&t.id) {
            self.sync_ui();
        }
    }

    fn snapshot(&self) -> Session {
        let mut tabs = Vec::new();
        for i in 0..self.view.n_pages() {
            if let Some(e) = self.entry_for_page(&self.view.nth_page(i)) {
                tabs.push(e.tab.meta());
            }
        }
        let active = self
            .view
            .selected_page()
            .map(|p| self.view.page_position(&p).max(0) as usize)
            .unwrap_or(0);
        let mut settings = self.settings.borrow().clone();
        let (w, h) = self.win.default_size();
        settings.maximized = self.win.is_maximized();
        if !settings.maximized && w > 0 && h > 0 {
            settings.width = w;
            settings.height = h;
        }
        Session { version: 1, active, tabs, settings }
    }

    // ===== backup =========================================================

    /// Current session and note contents, straight from the editors.
    fn backup_contents(&self) -> (Session, Vec<(String, String)>) {
        let session = self.snapshot();
        let notes = self
            .text_tabs()
            .into_iter()
            .filter(|t| t.needs_note_file())
            .map(|t| (t.id.clone(), t.text()))
            .collect();
        (session, notes)
    }

    fn backup(self: &Rc<Self>) {
        self.persist_all();
        let date = local_time("%Y-%m-%d").unwrap_or_else(|| "backup".into());
        let dialog = gtk::FileDialog::builder()
            .title("Back Up Notes")
            .modal(true)
            .initial_name(format!("quicknotes-backup-{date}.tar.gz"))
            .build();
        if let Some(home) = dirs::home_dir() {
            dialog.set_initial_folder(Some(&gio::File::for_path(home)));
        }
        let this = self.clone();
        glib::spawn_future_local(async move {
            let Ok(file) = dialog.save_future(Some(&this.win)).await else { return };
            let Some(path) = file.path() else { return };
            let (session, notes) = this.backup_contents();
            let count = session.tabs.len();
            let created = iso_now();
            let target = path.clone();
            let result = gio::spawn_blocking(move || crate::backup::write(&target, &created, &session, &notes))
                .await
                .unwrap_or_else(|_| Err("backup failed".into()));
            match result {
                Ok(()) => this.toast(&format!("Backup of {} saved to {}", tabs_count(count), tilde(&path))),
                Err(e) => this.toast(&format!("Backup failed: {e}")),
            }
        });
    }

    fn restore_backup(self: &Rc<Self>) {
        let filter = gtk::FileFilter::new();
        filter.set_name(Some("QuickNotes backups"));
        filter.add_pattern("*.tar.gz");
        filter.add_pattern("*.tgz");
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        let dialog = gtk::FileDialog::builder()
            .title("Restore Backup")
            .modal(true)
            .filters(&filters)
            .default_filter(&filter)
            .build();
        let this = self.clone();
        glib::spawn_future_local(async move {
            let Ok(file) = dialog.open_future(Some(&this.win)).await else { return };
            let Some(path) = file.path() else { return };
            let result = gio::spawn_blocking(move || crate::backup::read(&path))
                .await
                .unwrap_or_else(|_| Err("cannot read the backup".into()));
            let backup = match result {
                Ok(b) => b,
                Err(e) => {
                    this.toast(&format!("Restore failed: {e}"));
                    return;
                }
            };
            let created = backup.manifest.created.replace('T', " ");
            let created = created.get(..16).unwrap_or(&created);
            let dialog = adw::AlertDialog::new(
                Some("Restore Backup?"),
                Some(&format!(
                    "Backup from {created} with {}.\n\n\
                     Add: puts the tabs of the backup next to your current tabs.\n\n\
                     Replace: closes all current tabs and restores the backup including its \
                     settings. Your current notes are saved to an automatic backup first.",
                    tabs_count(backup.session.tabs.len())
                )),
            );
            dialog.add_responses(&[("cancel", "_Cancel"), ("add", "_Add"), ("replace", "_Replace")]);
            dialog.set_response_appearance("add", adw::ResponseAppearance::Suggested);
            dialog.set_response_appearance("replace", adw::ResponseAppearance::Destructive);
            dialog.set_default_response(Some("add"));
            dialog.set_close_response("cancel");
            match dialog.choose_future(&this.win).await.as_str() {
                "add" => this.import_add(backup),
                "replace" => this.import_replace(backup),
                _ => {}
            }
        });
    }

    /// Restores the tabs of a backup. Returns (restored, missing files).
    fn import_tabs(self: &Rc<Self>, backup: &crate::backup::Backup, merge: bool) -> (usize, usize) {
        let (mut restored, mut missing) = (0, 0);
        for meta in &backup.session.tabs {
            let content = backup.notes.get(&meta.id).cloned();
            if content.is_none() {
                match &meta.path {
                    // Notes without content (empty notes) are not worth restoring.
                    None => continue,
                    Some(p) if !p.exists() => {
                        missing += 1;
                        continue;
                    }
                    Some(_) => {}
                }
            }
            let mut meta = meta.clone();
            if merge {
                if self.already_open(&meta, content.as_deref()) {
                    continue;
                }
                if self.tabs.borrow().contains_key(&meta.id) {
                    meta.id = store::new_id();
                }
            }
            let id = meta.id.clone();
            if self.restore_tab(meta, content).is_some() {
                restored += 1;
                self.mark_dirty(&id);
            }
        }
        (restored, missing)
    }

    /// The tab of a backup is already open with the same content.
    fn already_open(&self, meta: &TabMeta, content: Option<&str>) -> bool {
        self.tabs.borrow().values().any(|e| match (&e.tab, content) {
            (Tab::Text(t), Some(c)) => *t.path.borrow() == meta.path && t.text() == c,
            (tab, None) => tab.path().is_some() && tab.path() == meta.path,
            _ => false,
        })
    }

    fn import_add(self: &Rc<Self>, backup: crate::backup::Backup) {
        let (restored, missing) = self.import_tabs(&backup, true);
        let mut msg = match restored {
            0 => "All tabs of the backup are already open".to_owned(),
            n => format!("Added {} from the backup", tabs_count(n)),
        };
        if missing > 0 {
            msg.push_str(&format!(" – {} not found", files_count(missing)));
        }
        self.toast(&msg);
        self.sync_ui();
    }

    fn import_replace(self: &Rc<Self>, backup: crate::backup::Backup) {
        // Safety net: the current state goes into an automatic backup first.
        self.persist_all();
        let dir = store::data_dir().join("backups");
        let stamp = local_time("%Y-%m-%d_%H-%M-%S").unwrap_or_else(store::new_id);
        let safety = dir.join(format!("before-restore-{stamp}.tar.gz"));
        let (session, notes) = self.backup_contents();
        if let Err(e) = std::fs::create_dir_all(&dir)
            .map_err(|e| e.to_string())
            .and_then(|_| crate::backup::write(&safety, &iso_now(), &session, &notes))
        {
            self.toast(&format!("Restore cancelled, the current notes could not be saved: {e}"));
            return;
        }

        self.bulk_close.set(true);
        for _ in 0..self.view.n_pages() {
            if self.view.n_pages() == 0 {
                break;
            }
            self.view.close_page(&self.view.nth_page(0));
        }
        self.bulk_close.set(false);
        self.closed.borrow_mut().clear();

        // Settings from the backup, but keep this computer's window geometry.
        let restored = backup.session.settings.clone();
        {
            let mut s = self.settings.borrow_mut();
            let (w, h, m) = (s.width, s.height, s.maximized);
            *s = restored.clone();
            (s.width, s.height, s.maximized) = (w, h, m);
        }
        *self.shared.plantuml_command.borrow_mut() = restored.plantuml_command.clone();
        self.set_dark(restored.dark);
        self.set_toggle("dark", true, restored.dark);
        self.set_toggle("ligatures", true, restored.ligatures);
        self.font_btn
            .set_font_desc(&gtk::pango::FontDescription::from_string(&restored.font_family));
        self.apply_font();

        let (count, missing) = self.import_tabs(&backup, false);
        if self.view.n_pages() == 0 {
            self.new_note();
        } else {
            let idx = (backup.session.active as i32).clamp(0, self.view.n_pages() - 1);
            self.view.set_selected_page(&self.view.nth_page(idx));
        }
        self.persist_all();
        self.sync_ui();
        let mut msg = format!("Restored {}. Previous notes saved to {}", tabs_count(count), tilde(&safety));
        if missing > 0 {
            msg.push_str(&format!(" – {} not found", files_count(missing)));
        }
        let toast = adw::Toast::builder().title(markup_escape(&msg)).timeout(8).build();
        self.toasts.add_toast(toast);
    }

    // ===== files ==========================================================

    fn is_large(&self, path: &Path) -> bool {
        let limit = self.settings.borrow().large_file_mb.max(1) * 1024 * 1024;
        path.metadata().map(|m| m.len() > limit).unwrap_or(false)
    }

    pub fn open_path(self: &Rc<Self>, path: PathBuf) {
        let path = std::fs::canonicalize(&path).unwrap_or(path);
        let existing = self
            .tabs
            .borrow()
            .values()
            .find(|e| e.tab.path().as_deref() == Some(path.as_path()))
            .map(|e| e.page.clone());
        if let Some(page) = existing {
            self.view.set_selected_page(&page);
            return;
        }
        if !path.is_file() {
            self.toast(&format!("Not a file: {}", path.display()));
            return;
        }

        // Replace an untouched empty note, like most editors do.
        let replace = self
            .current_text()
            .filter(|t| t.path.borrow().is_none() && t.is_empty() && t.custom_title.borrow().is_none())
            .and_then(|_| self.view.selected_page());

        if image_tab::is_image(&path) {
            match ImageTab::new(store::new_id(), path.clone()) {
                Ok(i) => {
                    self.add_tab(Tab::Image(i), true);
                }
                Err(e) => {
                    self.toast(&format!("Cannot open {}: {e}", path.display()));
                    return;
                }
            }
        } else {
            let t = EditorTab::new(store::new_id(), self.shared.clone());
            *t.path.borrow_mut() = Some(path.clone());
            let large = self.is_large(&path);
            t.set_large(large);
            t.set_wrap(false);
            t.set_language(langs::guess(Some(&path), &read_head(&path)).as_ref());
            self.add_tab(Tab::Text(t.clone()), true);
            if t.preview_kind().is_some_and(PreviewKind::auto_preview) && !large {
                t.set_preview(true);
                self.sync_ui();
            }
            self.load_into(&t, path, CursorTarget::Start);
            if large {
                self.toast("Large file – syntax highlighting and word wrap are off");
            }
        }

        if let Some(page) = replace {
            self.view.close_page(&page);
        }
        self.schedule_persist();
    }

    fn load_into(self: &Rc<Self>, t: &Rc<EditorTab>, path: PathBuf, cursor: CursorTarget) {
        let page = self.tabs.borrow().get(&t.id).map(|e| e.page.clone());
        if let Some(p) = &page {
            p.set_loading(true);
        }
        let this = self.clone();
        let t = t.clone();
        glib::spawn_future_local(async move {
            let result = t.clone().load_file(path.clone(), cursor).await;
            if let Some(p) = &page {
                p.set_loading(false);
            }
            if let Err(e) = result {
                this.toast(&format!("Cannot open {}: {e}", path.display()));
            }
            this.refresh_title(&t.id);
            if this.is_current(&t.id) {
                this.update_status();
            }
        });
    }

    fn open_dialog(self: &Rc<Self>) {
        let dialog = gtk::FileDialog::builder().title("Open Files").modal(true).build();
        if let Some(dir) = self.current().and_then(|t| t.path()).and_then(|p| p.parent().map(Path::to_owned)) {
            dialog.set_initial_folder(Some(&gio::File::for_path(dir)));
        }
        let this = self.clone();
        glib::spawn_future_local(async move {
            if let Ok(files) = dialog.open_multiple_future(Some(&this.win)).await {
                for file in files.iter::<gio::File>().flatten() {
                    if let Some(path) = file.path() {
                        this.open_path(path);
                    }
                }
            }
        });
    }

    fn save(self: &Rc<Self>) {
        let Some(t) = self.current_text() else { return };
        let path = t.path.borrow().clone();
        match path {
            Some(p) => match t.save_to(&p) {
                Ok(()) => {
                    self.toast(&format!("Saved {}", p.display()));
                    self.mark_dirty(&t.id);
                }
                Err(e) => self.toast(&format!("Save failed: {e}")),
            },
            None => self.save_as(),
        }
    }

    /// Exports the current Markdown/reStructuredText document.
    fn export(self: &Rc<Self>, format: ExportFormat) {
        let Some(t) = self.current_text() else { return };
        let Some(body) = t.document_html() else {
            self.toast("Export works for Markdown and reStructuredText");
            return;
        };
        let title = html::first_heading(&body).unwrap_or_else(|| t.title());
        let ext = match format {
            ExportFormat::Html => "html",
            ExportFormat::Pdf => "pdf",
        };
        let stem = t
            .path
            .borrow()
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| file_name_for(&title));
        let dialog = gtk::FileDialog::builder()
            .title(format!("Export as {}", ext.to_uppercase()))
            .modal(true)
            .initial_name(format!("{stem}.{ext}"))
            .build();
        if let Some(dir) = t.base_dir() {
            dialog.set_initial_folder(Some(&gio::File::for_path(dir)));
        }
        let document = html::standalone(&title, &body);
        let base_uri = t.base_uri();
        let this = self.clone();
        glib::spawn_future_local(async move {
            let Ok(file) = dialog.save_future(Some(&this.win)).await else { return };
            let Some(path) = file.path() else { return };
            match format {
                ExportFormat::Html => {
                    let result = std::fs::write(&path, document).map_err(|e| e.to_string());
                    this.export_done(&path, result);
                }
                ExportFormat::Pdf => {
                    this.toast("Exporting PDF…");
                    let weak = this.weak();
                    let target = path.clone();
                    crate::export::pdf(document, base_uri, path, move |result| {
                        if let Some(this) = weak.upgrade() {
                            this.export_done(&target, result);
                        }
                    });
                }
            }
        });
    }

    fn export_done(self: &Rc<Self>, path: &Path, result: Result<(), String>) {
        match result {
            Ok(()) => {
                let toast = adw::Toast::builder()
                    .title(markup_escape(&format!("Exported to {}", tilde(path))))
                    .button_label("Open")
                    .timeout(6)
                    .build();
                let file = gio::File::for_path(path);
                let win = self.win.clone();
                toast.connect_button_clicked(move |_| {
                    gtk::FileLauncher::new(Some(&file)).launch(Some(&win), None::<&gio::Cancellable>, |_| {});
                });
                self.toasts.add_toast(toast);
            }
            Err(e) => self.toast(&format!("Export failed: {e}")),
        }
    }

    fn save_as(self: &Rc<Self>) {
        let Some(t) = self.current_text() else { return };
        let dialog = gtk::FileDialog::builder().title("Save As").modal(true).build();
        let current_path = t.path.borrow().clone();
        match current_path {
            Some(p) => dialog.set_initial_file(Some(&gio::File::for_path(p))),
            None => {
                let ext = t.language_id().and_then(|l| langs::extension(&l)).unwrap_or_else(|| "txt".into());
                let base: String = t
                    .title()
                    .chars()
                    .map(|c| if c.is_alphanumeric() || "-_ .".contains(c) { c } else { '_' })
                    .collect::<String>()
                    .trim()
                    .trim_end_matches('…')
                    .to_owned();
                let base = if base.is_empty() { "note".into() } else { base };
                dialog.set_initial_name(Some(&format!("{base}.{ext}")));
            }
        }
        let this = self.clone();
        glib::spawn_future_local(async move {
            let Ok(file) = dialog.save_future(Some(&this.win)).await else { return };
            let Some(path) = file.path() else { return };
            match t.save_to(&path) {
                Ok(()) => {
                    if !t.language_locked.get() {
                        if let Some(lang) = langs::guess(Some(&path), &t.text()) {
                            t.set_language(Some(&lang));
                        }
                    }
                    this.toast(&format!("Saved {}", path.display()));
                    this.refresh_title(&t.id);
                    this.mark_dirty(&t.id);
                    this.sync_ui();
                }
                Err(e) => this.toast(&format!("Save failed: {e}")),
            }
        });
    }

    fn reload(self: &Rc<Self>) {
        let Some(t) = self.current_text() else { return };
        let Some(path) = t.path.borrow().clone() else {
            self.toast("This note has no file to reload");
            return;
        };
        if !t.buffer.is_modified() {
            t.set_large(self.is_large(&path));
            self.load_into(&t, path, t.cursor_target());
            return;
        }
        let dialog = adw::AlertDialog::new(
            Some("Discard Changes and Reload?"),
            Some("Your unsaved changes to this file will be lost."),
        );
        dialog.add_responses(&[("cancel", "_Cancel"), ("reload", "_Reload")]);
        dialog.set_response_appearance("reload", adw::ResponseAppearance::Destructive);
        dialog.set_close_response("cancel");
        let this = self.clone();
        glib::spawn_future_local(async move {
            if dialog.choose_future(&this.win).await == "reload" {
                this.load_into(&t, path, t.cursor_target());
            }
        });
    }

    fn rename(self: &Rc<Self>, page: &adw::TabPage) {
        let Some(Entry { tab: Tab::Text(t), .. }) = self.entry_for_page(page) else {
            self.toast("Only text tabs can be renamed");
            return;
        };
        let entry = gtk::Entry::builder()
            .text(t.custom_title.borrow().clone().unwrap_or_else(|| t.title()))
            .activates_default(true)
            .build();
        let dialog = adw::AlertDialog::new(
            Some("Rename Tab"),
            Some("Leave empty to name the tab after its first line or file."),
        );
        dialog.set_extra_child(Some(&entry));
        dialog.add_responses(&[("cancel", "_Cancel"), ("rename", "_Rename")]);
        dialog.set_response_appearance("rename", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("rename"));
        dialog.set_close_response("cancel");
        let this = self.clone();
        glib::spawn_future_local(async move {
            if dialog.choose_future(&this.win).await == "rename" {
                let name = entry.text().trim().to_owned();
                *t.custom_title.borrow_mut() = (!name.is_empty()).then_some(name);
                this.refresh_title(&t.id);
                this.mark_dirty(&t.id);
            }
        });
    }

    // ===== tools ==========================================================

    fn apply_tool(self: &Rc<Self>, tool: &str) {
        let Some(t) = self.current_text() else { return };
        let buf = &t.buffer;
        let (start, end, selection) = match buf.selection_bounds() {
            Some((s, e)) => (s, e, true),
            None => {
                let (s, e) = buf.bounds();
                (s, e, false)
            }
        };
        let input = buf.text(&start, &end, true).to_string();
        let lang = t.language_id();
        match format::apply(tool, &input, lang.as_deref()) {
            Ok((output, out_lang)) => {
                if output == input {
                    self.toast("Nothing to change");
                    return;
                }
                let offset = start.offset();
                let (mut s, mut e) = (start, end);
                buf.begin_user_action();
                buf.delete(&mut s, &mut e);
                buf.insert(&mut s, &output);
                buf.end_user_action();
                if selection {
                    let s = buf.iter_at_offset(offset);
                    let e = buf.iter_at_offset(offset + output.chars().count() as i32);
                    buf.select_range(&s, &e);
                } else {
                    buf.place_cursor(&buf.start_iter());
                    if let Some(l) = out_lang {
                        if !t.language_locked.get() || t.path.borrow().is_none() {
                            t.set_language_id(Some(l));
                            self.sync_ui();
                        }
                    }
                }
            }
            Err(e) => self.toast(&e),
        }
    }

    // ===== appearance =====================================================

    fn zoom(self: &Rc<Self>, delta: f64) {
        {
            let mut s = self.settings.borrow_mut();
            s.font_size = (s.font_size + delta).clamp(MIN_FONT, MAX_FONT);
        }
        self.apply_font();
        self.schedule_persist();
    }

    fn apply_font(&self) {
        let s = self.settings.borrow();
        let family = s.font_family.replace(['"', '\\', ';', '}'], "");
        let features = if s.ligatures { "normal" } else { "\"liga\" 0, \"calt\" 0, \"dlig\" 0" };
        self.editor_css.load_from_string(&format!(
            ".editor-view {{ font-family: \"{family}\", monospace; font-size: {}pt; font-feature-settings: {features}; }}",
            s.font_size
        ));
        self.shared.zoom.set(s.font_size / BASE_FONT);
        self.zoom_btn.set_label(&format!("{} pt", s.font_size));
        drop(s);
        for t in self.text_tabs() {
            t.apply_zoom();
        }
    }

    fn set_dark(self: &Rc<Self>, dark: bool) {
        self.settings.borrow_mut().dark = dark;
        self.shared.dark.set(dark);
        adw::StyleManager::default().set_color_scheme(if dark {
            adw::ColorScheme::ForceDark
        } else {
            adw::ColorScheme::ForceLight
        });
        for t in self.text_tabs() {
            t.apply_theme();
        }
        self.apply_theme_ui();
        self.schedule_persist();
    }

    fn apply_theme_ui(&self) {
        let dark = self.settings.borrow().dark;
        self.theme_btn.set_icon_name(if dark { "weather-clear-symbolic" } else { "weather-clear-night-symbolic" });
        self.theme_btn
            .set_tooltip_text(Some(if dark { "Switch to light theme" } else { "Switch to dark theme" }));
    }

    fn toast(&self, text: &str) {
        let toast = adw::Toast::new(&markup_escape(text));
        toast.set_timeout(4);
        self.toasts.add_toast(toast);
    }

    // ===== UI sync ========================================================

    fn sync_ui(self: &Rc<Self>) {
        let has_tabs = self.view.n_pages() > 0;
        self.stack.set_visible_child_name(if has_tabs { "tabs" } else { "empty" });

        self.guard.set(true);
        match self.current() {
            Some(Tab::Text(t)) => {
                let lang = t.language_id();
                let idx = self.lang_ids.iter().position(|l| *l == lang).unwrap_or(0);
                self.lang_drop.set_selected(idx as u32);
                self.lang_drop.set_visible(true);
                self.set_toggle("preview", t.preview_kind().is_some(), t.preview_visible());
                self.set_toggle("wrap", true, t.wrap());
                let exportable = t.preview_kind().is_some_and(PreviewKind::is_document);
                self.set_enabled("export-html", exportable);
                self.set_enabled("export-pdf", exportable);
            }
            _ => {
                self.lang_drop.set_visible(false);
                self.set_toggle("preview", false, false);
                self.set_toggle("wrap", false, false);
                self.set_enabled("export-html", false);
                self.set_enabled("export-pdf", false);
            }
        }
        self.guard.set(false);

        // Search highlighting only in the visible tab.
        let searching = self.search.revealer.reveals_child();
        let current_id = self.current_text().map(|t| t.id.clone());
        for t in self.text_tabs() {
            let active = searching && current_id.as_deref() == Some(t.id.as_str());
            if active {
                self.ensure_search_context(&t);
            }
            t.set_search_highlight(active);
        }

        self.update_window_title();
        self.update_status();
    }

    fn update_window_title(&self) {
        let subtitle = self
            .current()
            .and_then(|t| t.path())
            .map(|p| tilde(&p))
            .unwrap_or_default();
        self.title.set_subtitle(&subtitle);
        let title = self.current().map(|t| t.title()).unwrap_or_default();
        self.title.set_title(if title.is_empty() { "QuickNotes" } else { &title });
        self.win.set_title(Some(&if title.is_empty() {
            "QuickNotes".to_owned()
        } else {
            format!("{title} – QuickNotes")
        }));
    }

    fn update_status(&self) {
        match self.current() {
            Some(Tab::Text(t)) => {
                let mut info = vec![match t.path.borrow().as_ref() {
                    Some(p) => tilde(p),
                    None => "Note".into(),
                }];
                info.push(format!("{} lines", t.buffer.line_count()));
                if t.large.get() {
                    info.push("large file, highlighting off".into());
                }
                self.status_info.set_text(&info.join("  ·  "));
                self.update_position();
            }
            Some(Tab::Image(i)) => {
                self.status_info.set_text(&format!("{}  ·  {}", tilde(&i.path), i.info()));
                self.status_pos.set_text("");
            }
            None => {
                self.status_info.set_text("");
                self.status_pos.set_text("");
            }
        }
    }

    fn update_position(&self) {
        let Some(t) = self.current_text() else { return };
        let buf = &t.buffer;
        let iter = buf.iter_at_mark(&buf.get_insert());
        let mut text = format!("Ln {}, Col {}", iter.line() + 1, iter.line_offset() + 1);
        if let Some((s, e)) = buf.selection_bounds() {
            text.push_str(&format!("  ({} selected)", e.offset() - s.offset()));
        }
        self.status_pos.set_text(&text);
        if self.search.revealer.reveals_child() {
            self.update_match_count();
        }
    }

    // ===== search =========================================================

    fn setup_search(self: &Rc<Self>) {
        let s = &self.search;
        let weak = self.weak();
        s.find.connect_search_changed(move |e| {
            if let Some(this) = weak.upgrade() {
                let text = e.text();
                this.search
                    .settings
                    .set_search_text((!text.is_empty()).then_some(text.as_str()));
                this.find_step(true, true);
            }
        });
        let weak = self.weak();
        s.find.connect_activate(move |_| {
            if let Some(this) = weak.upgrade() {
                this.find_step(true, false);
            }
        });
        let weak = self.weak();
        s.find.connect_next_match(move |_| {
            if let Some(this) = weak.upgrade() {
                this.find_step(true, false);
            }
        });
        let weak = self.weak();
        s.find.connect_previous_match(move |_| {
            if let Some(this) = weak.upgrade() {
                this.find_step(false, false);
            }
        });
        let weak = self.weak();
        s.find.connect_stop_search(move |_| {
            if let Some(this) = weak.upgrade() {
                this.close_search();
            }
        });

        // Shift+Enter searches backwards; Escape in the replace field closes.
        for widget in [s.find.clone().upcast::<gtk::Widget>(), s.replace.clone().upcast()] {
            let keys = gtk::EventControllerKey::new();
            let weak = self.weak();
            keys.connect_key_pressed(move |_, key, _, state| {
                let Some(this) = weak.upgrade() else { return glib::Propagation::Proceed };
                match key {
                    gdk::Key::Return | gdk::Key::KP_Enter if state.contains(gdk::ModifierType::SHIFT_MASK) => {
                        this.find_step(false, false);
                        glib::Propagation::Stop
                    }
                    gdk::Key::Escape => {
                        this.close_search();
                        glib::Propagation::Stop
                    }
                    _ => glib::Propagation::Proceed,
                }
            });
            widget.add_controller(keys);
        }

        let weak = self.weak();
        s.replace.connect_activate(move |_| {
            if let Some(this) = weak.upgrade() {
                this.replace_one();
            }
        });

        let row = s.revealer.child().and_downcast::<gtk::Box>().expect("search box");
        let buttons: Vec<gtk::Widget> = descendants(&row)
            .into_iter()
            .filter(|w| w.is::<gtk::Button>() && !w.is::<gtk::ToggleButton>())
            .collect();
        for b in buttons {
            let b = b.downcast::<gtk::Button>().unwrap();
            let weak = self.weak();
            b.connect_clicked(move |b| {
                let Some(this) = weak.upgrade() else { return };
                match b.widget_name().as_str() {
                    "prev" => this.find_step(false, false),
                    "next" => this.find_step(true, false),
                    "replace" => this.replace_one(),
                    "replace-all" => this.replace_all(),
                    "close" => this.close_search(),
                    _ => {}
                }
            });
        }

        // Re-run the search when options change.
        for prop in ["case-sensitive", "regex-enabled", "at-word-boundaries"] {
            let weak = self.weak();
            s.settings.connect_notify_local(Some(prop), move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.update_match_count();
                }
            });
        }
    }

    fn ensure_search_context(self: &Rc<Self>, t: &Rc<EditorTab>) -> sourceview5::SearchContext {
        let (ctx, created) = t.search_context(&self.search.settings);
        if created {
            let weak = self.weak();
            ctx.connect_occurrences_count_notify(move |_| {
                if let Some(this) = weak.upgrade() {
                    this.update_match_count();
                }
            });
        }
        ctx
    }

    fn open_search(self: &Rc<Self>, with_replace: bool) {
        let Some(t) = self.current_text() else { return };
        if let Some((s, e)) = t.buffer.selection_bounds() {
            if s.line() == e.line() {
                self.search.find.set_text(&t.buffer.text(&s, &e, false));
            }
        }
        self.search.replace_row.set_visible(with_replace);
        self.search.revealer.set_reveal_child(true);
        self.search.find.grab_focus();
        self.search.find.select_region(0, -1);
        self.sync_ui();
        self.update_match_count();
    }

    fn close_search(self: &Rc<Self>) {
        self.search.revealer.set_reveal_child(false);
        for t in self.text_tabs() {
            t.set_search_highlight(false);
        }
        if let Some(t) = self.current_text() {
            t.view.grab_focus();
        }
    }

    /// Moves to the next/previous match. `from_selection_start` keeps the
    /// current match while typing (incremental search).
    fn find_step(self: &Rc<Self>, forward: bool, from_selection_start: bool) {
        let Some(t) = self.current_text() else { return };
        if self.search.settings.search_text().is_none() {
            self.update_match_count();
            return;
        }
        let ctx = self.ensure_search_context(&t);
        if self.search.revealer.reveals_child() {
            ctx.set_highlight(true);
        }
        let buf = &t.buffer;
        let (s, e) = buf.selection_bounds().unwrap_or_else(|| {
            let i = buf.iter_at_mark(&buf.get_insert());
            (i, i)
        });
        let found = if forward {
            ctx.forward(if from_selection_start { &s } else { &e })
        } else {
            ctx.backward(&s)
        };
        if let Some((ms, me, _)) = found {
            buf.select_range(&ms, &me);
            t.view.scroll_to_mark(&buf.get_insert(), 0.15, false, 0.0, 0.0);
        }
        self.update_match_count();
    }

    fn replace_one(self: &Rc<Self>) {
        let Some(t) = self.current_text() else { return };
        let ctx = self.ensure_search_context(&t);
        if let Some((mut s, mut e)) = t.buffer.selection_bounds() {
            if ctx.occurrence_position(&s, &e) > 0 {
                let _ = ctx.replace(&mut s, &mut e, &self.search.replace.text());
            }
        }
        self.find_step(true, false);
    }

    fn replace_all(self: &Rc<Self>) {
        let Some(t) = self.current_text() else { return };
        let ctx = self.ensure_search_context(&t);
        let count = ctx.occurrences_count();
        match ctx.replace_all(&self.search.replace.text()) {
            Ok(_) if count > 0 => self.toast(&format!("Replaced {count} occurrences")),
            Ok(_) => self.toast("Nothing replaced"),
            Err(e) => self.toast(&format!("Replace failed: {e}")),
        }
    }

    fn update_match_count(&self) {
        let label = &self.search.count;
        let Some(t) = self.current_text() else {
            label.set_text("");
            return;
        };
        if self.search.settings.search_text().is_none() {
            label.set_text("");
            return;
        }
        let (ctx, _) = t.search_context(&self.search.settings);
        let total = ctx.occurrences_count();
        let text = if total < 0 {
            "…".to_owned()
        } else if total == 0 {
            "No matches".to_owned()
        } else {
            match t.buffer.selection_bounds().map(|(s, e)| ctx.occurrence_position(&s, &e)) {
                Some(pos) if pos > 0 => format!("{pos} of {total}"),
                _ => format!("{total} matches"),
            }
        };
        label.set_text(&text);
    }
}

// ===== drag and drop =====================================================

const URI_LIST: &str = "text/uri-list";
/// Text formats in order of preference (X11 target names included).
const TEXT_MIMES: &[&str] = &["text/plain;charset=utf-8", "UTF8_STRING", "text/plain"];

#[derive(Clone)]
enum DropKind {
    /// Opens dropped files.
    Files,
    /// Opens dropped files, inserts dropped text.
    Editor(Weak<EditorTab>),
}

type DropSlot = Rc<RefCell<Option<gtk::DropTargetAsync>>>;

/// Reads the dropped data as `mime`, trying the direct X11 path first.
async fn read_drop_data(drop: &gdk::Drop, x11_window: Option<u32>, mime: &str) -> Option<String> {
    let decode = |data: &[u8]| String::from_utf8_lossy(data).trim_end_matches('\0').to_owned();
    if let Some(xid) = x11_window {
        let target = mime.to_owned();
        if let Ok(Ok(data)) = gio::spawn_blocking(move || crate::xdnd::read(xid, &target)).await {
            if !data.is_empty() {
                return Some(decode(&data));
            }
        }
    }
    let read = async {
        let (stream, _) = drop.read_future(&[mime], glib::Priority::DEFAULT).await.ok()?;
        let mut data = Vec::new();
        loop {
            match stream.read_bytes_future(64 * 1024, glib::Priority::DEFAULT).await {
                Ok(chunk) if !chunk.is_empty() => data.extend_from_slice(&chunk),
                _ => break,
            }
        }
        Some(decode(&data))
    };
    glib::future_with_timeout(Duration::from_secs(3), read).await.ok().flatten()
}

/// The URIs of a `text/uri-list` payload, one per line (for inserting links).
fn uri_list_as_text(list: &str) -> String {
    list.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parses a `text/uri-list` payload (RFC 2483) into local paths.
fn parse_uri_list(list: &str) -> Vec<PathBuf> {
    list.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|uri| gio::File::for_uri(uri).path())
        .collect()
}

// ===== construction helpers ==============================================

fn build_search_ui() -> SearchUi {
    let settings = sourceview5::SearchSettings::new();
    settings.set_wrap_around(true);

    let find = gtk::SearchEntry::builder()
        .placeholder_text("Find")
        .hexpand(true)
        .build();
    let replace = gtk::Entry::builder()
        .placeholder_text("Replace with")
        .hexpand(true)
        .build();

    let named_button = |name: &str, icon: Option<&str>, label: Option<&str>, tip: &str| {
        let b = gtk::Button::builder().tooltip_text(tip).build();
        if let Some(i) = icon {
            b.set_icon_name(i);
        }
        if let Some(l) = label {
            b.set_label(l);
        }
        b.set_widget_name(name);
        b
    };
    let toggle = |label: &str, tip: &str, prop: &str| {
        let b = gtk::ToggleButton::builder().label(label).tooltip_text(tip).build();
        b.add_css_class("flat");
        b.bind_property("active", &settings, prop).sync_create().bidirectional().build();
        b
    };

    let nav = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    nav.add_css_class("linked");
    nav.append(&named_button("prev", Some("go-up-symbolic"), None, "Previous match (Shift+Enter)"));
    nav.append(&named_button("next", Some("go-down-symbolic"), None, "Next match (Enter)"));

    let count = gtk::Label::builder().width_chars(10).xalign(0.5).build();
    count.add_css_class("dim-label");
    count.add_css_class("numeric");

    let close = named_button("close", Some("window-close-symbolic"), None, "Close (Esc)");
    close.add_css_class("flat");

    let find_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    find_row.append(&find);
    find_row.append(&nav);
    find_row.append(&toggle("Aa", "Match case", "case-sensitive"));
    find_row.append(&toggle("W", "Whole words", "at-word-boundaries"));
    find_row.append(&toggle(".*", "Regular expression", "regex-enabled"));
    find_row.append(&count);
    find_row.append(&close);

    let replace_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    replace_row.append(&replace);
    replace_row.append(&named_button("replace", None, Some("Replace"), "Replace current match (Enter)"));
    replace_row.append(&named_button("replace-all", None, Some("Replace All"), "Replace all matches"));
    // Align the replace row with the find entry.
    replace_row.set_margin_end(0);

    let column = gtk::Box::new(gtk::Orientation::Vertical, 6);
    column.add_css_class("search-panel");
    column.append(&find_row);
    column.append(&replace_row);

    let revealer = gtk::Revealer::builder()
        .child(&column)
        .transition_type(gtk::RevealerTransitionType::SlideDown)
        .reveal_child(false)
        .build();

    SearchUi { revealer, find, replace, replace_row, count, settings }
}

fn main_menu() -> gio::Menu {
    let menu = gio::Menu::new();
    let section = |items: &[(&str, &str)]| {
        let s = gio::Menu::new();
        for (label, action) in items {
            s.append(Some(label), Some(action));
        }
        s
    };
    menu.append_section(
        None,
        &section(&[
            ("New Note", "win.new-note"),
            ("Open File…", "win.open"),
            ("Save", "win.save"),
            ("Save As…", "win.save-as"),
            ("Reload from Disk", "win.reload"),
        ]),
    );
    menu.append_section(
        None,
        &section(&[("Export as HTML…", "win.export-html"), ("Export as PDF…", "win.export-pdf")]),
    );
    menu.append_section(
        None,
        &section(&[
            ("Find…", "win.find"),
            ("Find and Replace…", "win.replace"),
            ("Rename Tab…", "win.rename"),
            ("Reopen Closed Tab", "win.reopen-closed"),
        ]),
    );
    menu.append_section(
        None,
        &section(&[("Word Wrap", "win.wrap"), ("Preview", "win.preview"), ("Dark Theme", "win.dark")]),
    );
    menu.append_section(
        None,
        &section(&[
            ("Larger Font", "win.zoom-in"),
            ("Smaller Font", "win.zoom-out"),
            ("Reset Font Size", "win.zoom-reset"),
            ("Font Ligatures", "win.ligatures"),
        ]),
    );
    menu.append_section(
        None,
        &section(&[
            ("Back Up Notes…", "win.backup"),
            ("Restore Backup…", "win.restore-backup"),
            ("Open Notes Folder", "win.open-data-dir"),
        ]),
    );
    menu.append_section(None, &section(&[("About QuickNotes", "win.about")]),
    );
    menu
}

fn tools_menu() -> gio::Menu {
    let menu = gio::Menu::new();
    let items = |list: &[(&str, &str)]| {
        let m = gio::Menu::new();
        for (label, tool) in list {
            m.append(Some(label), Some(&format!("win.tool::{tool}")));
        }
        m
    };
    menu.append_section(
        Some("Applies to the selection or the whole tab"),
        &items(&[("Beautify (auto-detect)", "beautify"), ("Minify (auto-detect)", "minify")]),
    );
    let more = gio::Menu::new();
    more.append_submenu(
        Some("Format"),
        &items(&[
            ("Pretty JSON", "json-pretty"),
            ("Minify JSON", "json-minify"),
            ("Pretty XML", "xml-pretty"),
            ("Minify XML", "xml-minify"),
            ("Pretty YAML", "yaml-pretty"),
            ("Pretty SQL", "sql-pretty"),
        ]),
    );
    more.append_submenu(
        Some("Convert"),
        &items(&[("JSON → YAML", "json-to-yaml"), ("YAML → JSON", "yaml-to-json")]),
    );
    more.append_submenu(
        Some("Lines"),
        &items(&[
            ("Sort", "sort-lines"),
            ("Sort (case-insensitive)", "sort-lines-ci"),
            ("Reverse", "reverse-lines"),
            ("Remove Duplicates", "unique-lines"),
            ("Remove Empty Lines", "remove-empty-lines"),
            ("Trim Trailing Whitespace", "trim-trailing"),
        ]),
    );
    more.append_submenu(
        Some("Encode / Decode"),
        &items(&[
            ("Base64 Encode", "base64-encode"),
            ("Base64 Decode", "base64-decode"),
            ("URL Encode", "url-encode"),
            ("URL Decode", "url-decode"),
            ("JSON String Escape", "json-escape"),
            ("JSON String Unescape", "json-unescape"),
        ]),
    );
    more.append_submenu(Some("Case"), &items(&[("UPPERCASE", "upper"), ("lowercase", "lower")]));
    menu.append_section(None, &more);
    menu
}

// ===== small helpers =====================================================

fn descendants(widget: &impl IsA<gtk::Widget>) -> Vec<gtk::Widget> {
    let mut out = Vec::new();
    let mut child = widget.first_child();
    while let Some(c) = child {
        out.extend(descendants(&c));
        child = c.next_sibling();
        out.push(c);
    }
    out
}

fn tabs_count(n: usize) -> String {
    if n == 1 { "1 tab".into() } else { format!("{n} tabs") }
}

fn files_count(n: usize) -> String {
    if n == 1 { "1 file".into() } else { format!("{n} files") }
}

/// Local time formatted with a `strftime`-like pattern.
fn local_time(pattern: &str) -> Option<String> {
    glib::DateTime::now_local().ok()?.format(pattern).ok().map(String::from)
}

fn iso_now() -> String {
    glib::DateTime::now_local()
        .ok()
        .and_then(|d| d.format_iso8601().ok())
        .map(String::from)
        .unwrap_or_default()
}

#[derive(Clone, Copy)]
enum ExportFormat {
    Html,
    Pdf,
}

/// A file name (without extension) derived from a document title.
fn file_name_for(title: &str) -> String {
    let name: String = title
        .chars()
        .map(|c| if c.is_alphanumeric() || "-_ .".contains(c) { c } else { '_' })
        .collect();
    let name = name.trim().trim_end_matches('…').trim().to_owned();
    if name.is_empty() { "document".into() } else { name }
}

fn read_head(path: &Path) -> String {
    use std::io::Read;
    let mut buf = vec![0u8; 4096];
    let n = std::fs::File::open(path).and_then(|mut f| f.read(&mut buf)).unwrap_or(0);
    String::from_utf8_lossy(&buf[..n]).into_owned()
}

fn tilde(path: &Path) -> String {
    let s = path.to_string_lossy();
    match dirs::home_dir().map(|h| h.to_string_lossy().into_owned()) {
        Some(home) if s.starts_with(&home) => format!("~{}", &s[home.len()..]),
        _ => s.into_owned(),
    }
}

fn markup_escape(s: &str) -> String {
    glib::markup_escape_text(s).to_string()
}

#[cfg(test)]
mod tests {
    use super::parse_uri_list;
    use std::path::PathBuf;

    #[test]
    fn uri_list_text() {
        assert_eq!(super::uri_list_as_text("# c\r\nhttps://a.b/c\r\nhttps://d.e\r\n"), "https://a.b/c\nhttps://d.e");
    }

    #[test]
    fn parses_uri_lists() {
        let list = "# comment\r\nfile:///tmp/a%20b.md\r\nfile:///tmp/test.php\r\nhttps://example.com/x\r\n";
        assert_eq!(
            parse_uri_list(list),
            vec![PathBuf::from("/tmp/a b.md"), PathBuf::from("/tmp/test.php")]
        );
    }
}
