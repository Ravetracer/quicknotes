//! Main window: tab management, actions, menus, search bar, status bar and
//! session persistence.

use crate::editor::{CursorTarget, EditorTab, PreviewKind, Shared};
use crate::image_tab::{self, ImageTab};
use crate::store::{self, Session, Settings, TabKind, TabMeta};
use crate::geocaching::{self, Field};
use crate::presenter::{self, Presentation};
use crate::prefs::{self, MAX_FONT, MIN_FONT};
use crate::tab_list::TabList;
use crate::themes::{self, Theme};
use crate::{checklist, format, html, langs, slides, stats, table, APP_ID};
use adw::prelude::*;
use gtk::{gdk, gio, glib};
use sourceview5::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};
use std::time::Duration;

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
    /// Words, characters and paragraphs of the document or selection.
    status_counts: gtk::Label,
    counts_pending: Cell<bool>,
    zoom_btn: gtk::Button,
    lang_drop: gtk::DropDown,
    lang_ids: Vec<Option<String>>,
    guard: Cell<bool>,
    search: SearchUi,
    closed: RefCell<Vec<(TabMeta, Option<String>)>>,
    dirty: RefCell<HashSet<String>>,
    save_pending: Cell<bool>,
    scroll_acc: Cell<f64>,
    menu_page: RefCell<Option<adw::TabPage>>,
    /// Closing all tabs for a restore: no confirmations, no undo.
    bulk_close: Cell<bool>,
    present_btn: adw::SplitButton,
    presentation: RefCell<Option<Rc<Presentation>>>,
    tab_bar: adw::TabBar,
    /// Vertical tabs (sidebar) and the pane holding it next to the tabs.
    tab_list: Rc<TabList>,
    content: gtk::Paned,
    toolbar: adw::ToolbarView,
    /// Tab colours (generated from the settings).
    tab_css: gtk::CssProvider,
    /// libadwaita colour overrides of the current theme.
    theme_css: gtk::CssProvider,
    /// Distraction-free mode: fullscreen editor without bars and sidebar.
    focus_mode: Cell<bool>,
    /// The header bar's main menu; rebuilt when the menu context changes.
    main_menu: gio::Menu,
    menu_context: Cell<Option<MenuContext>>,
    /// Style each text tab's context menu was last built for.
    tab_styles: RefCell<HashMap<String, (TabStyle, bool)>>,
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
        crate::i18n::init(&settings.language);
        let theme = themes::find(&settings.theme, settings.dark);

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
        header.pack_start(&icon_button("tab-new-symbolic", t!("New note (Ctrl+N)"), "win.new-note"));
        header.pack_start(&icon_button("document-open-symbolic", t!("Open file (Ctrl+O)"), "win.open"));
        header.pack_start(&icon_button("document-save-symbolic", t!("Save to file (Ctrl+S)"), "win.save"));

        let main_menu = gio::Menu::new();
        fill_main_menu(&main_menu, MenuContext { style: None, has_file: false, emmet: false });
        let menu_btn = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .menu_model(&main_menu)
            .primary(true)
            .tooltip_text(t!("Main menu"))
            .build();
        let tools_btn = gtk::MenuButton::builder()
            .icon_name("format-justify-left-symbolic")
            .menu_model(&tools_menu())
            .tooltip_text(t!("Format & tools"))
            .build();
        let preview_btn = gtk::ToggleButton::builder()
            .icon_name("view-dual-symbolic")
            .tooltip_text(t!("Toggle preview (Ctrl+Shift+V)"))
            .action_name("win.preview")
            .build();
        let find_btn = icon_button("edit-find-symbolic", t!("Find (Ctrl+F)"), "win.find");
        let present_menu = gio::Menu::new();
        present_menu.append(Some(t!("From Beginning")), Some("win.present"));
        present_menu.append(Some(t!("From Current Slide")), Some("win.present-current"));
        present_menu.append(Some(t!("In a Window")), Some("win.present-window"));
        let present_btn = adw::SplitButton::builder()
            .icon_name("media-playback-start-symbolic")
            .tooltip_text(t!("Start presentation (Ctrl+F5)"))
            .dropdown_tooltip(t!("Presentation options"))
            .action_name("win.present")
            .menu_model(&present_menu)
            .visible(false)
            .build();
        header.pack_end(&menu_btn);
        header.pack_end(&tools_btn);
        header.pack_end(&preview_btn);
        header.pack_end(&find_btn);
        header.pack_end(&present_btn);

        // ----- tabs ------------------------------------------------------
        let view = adw::TabView::new();
        let tab_menu = gio::Menu::new();
        tab_menu.append(Some(t!("Rename…")), Some("win.tab-rename"));
        tab_menu.append(Some(t!("Copy Path")), Some("win.tab-copy-path"));
        tab_menu.append(Some(t!("Close Other Tabs")), Some("win.tab-close-others"));
        tab_menu.append(Some(t!("Close")), Some("win.tab-close"));
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
        let status_counts = gtk::Label::builder().margin_start(12).margin_end(12).build();
        let zoom_btn = gtk::Button::builder()
            .label("12 pt")
            .tooltip_text(t!("Font size – Ctrl+wheel or Ctrl+/−, click to reset"))
            .action_name("win.zoom-reset")
            .build();
        zoom_btn.add_css_class("flat");
        let zoom_step = |label: &str, tip: &str, action: &str| {
            let b = gtk::Button::builder().label(label).tooltip_text(tip).action_name(action).build();
            b.add_css_class("flat");
            b
        };
        let zoom_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        zoom_box.append(&zoom_step("−", t!("Smaller font (Ctrl+−)"), "win.zoom-out"));
        zoom_box.append(&zoom_btn);
        zoom_box.append(&zoom_step("+", t!("Larger font (Ctrl++)"), "win.zoom-in"));

        let languages = langs::all();
        let mut lang_ids: Vec<Option<String>> = vec![None];
        let mut names: Vec<String> = vec![t!("Plain Text").into()];
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
        lang_drop.set_tooltip_text(Some(t!("Syntax")));
        lang_drop.add_css_class("flat");
        enter_picks_first_match(&lang_drop);

        status.append(&status_info);
        status.append(&status_counts);
        status.append(&status_pos);
        status.append(&zoom_box);
        status.append(&lang_drop);

        // ----- layout ----------------------------------------------------
        let empty = adw::StatusPage::builder()
            .icon_name(APP_ID)
            .title(t!("No Open Notes"))
            .description(t!("Create a note with Ctrl+N or open a file with Ctrl+O.\nFiles can also be dropped here."))
            .build();
        let new_btn = gtk::Button::builder()
            .label(t!("New Note"))
            .action_name("win.new-note")
            .halign(gtk::Align::Center)
            .build();
        new_btn.add_css_class("pill");
        new_btn.add_css_class("suggested-action");
        empty.set_child(Some(&new_btn));

        let stack = gtk::Stack::new();
        stack.add_named(&view, Some("tabs"));
        stack.add_named(&empty, Some("empty"));

        // Vertical tabs: a sidebar next to the tabs (hidden for horizontal tabs).
        let menu_target: Rc<RefCell<Option<Weak<Window>>>> = Rc::default();
        let target = menu_target.clone();
        let tab_list = TabList::new(
            &view,
            Box::new(move |page| {
                if let Some(this) = target.borrow().as_ref().and_then(Weak::upgrade) {
                    this.set_menu_page(page);
                }
            }),
        );
        let content = gtk::Paned::new(gtk::Orientation::Horizontal);
        content.set_start_child(Some(&tab_list.root));
        content.set_end_child(Some(&stack));
        content.set_resize_start_child(false);
        content.set_shrink_start_child(false);
        content.set_shrink_end_child(false);
        content.set_position(settings.sidebar_width.max(120));

        let toasts = adw::ToastOverlay::new();
        toasts.set_child(Some(&content));

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
        let tab_css = gtk::CssProvider::new();
        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().expect("no display"),
            &tab_css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        let theme_css = gtk::CssProvider::new();
        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().expect("no display"),
            &theme_css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let shared = Rc::new(Shared {
            theme: Cell::new(theme),
            emmet_tab: Cell::new(settings.emmet_tab),
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
            status_counts,
            counts_pending: Cell::new(false),
            zoom_btn,
            lang_drop,
            lang_ids,
            guard: Cell::new(false),
            search,
            closed: RefCell::new(Vec::new()),
            dirty: RefCell::new(HashSet::new()),
            save_pending: Cell::new(false),
            scroll_acc: Cell::new(0.0),
            menu_page: RefCell::new(None),
            bulk_close: Cell::new(false),
            present_btn,
            presentation: RefCell::new(None),
            tab_bar,
            tab_list,
            content,
            toolbar: toolbar.clone(),
            tab_css,
            theme_css,
            focus_mode: Cell::new(false),
            main_menu,
            menu_context: Cell::new(None),
            tab_styles: RefCell::new(HashMap::new()),
        });

        *menu_target.borrow_mut() = Some(this.weak());
        this.setup_actions(app);
        this.setup_signals(&toolbar);
        this.setup_search();
        this.apply_font();
        this.apply_tab_colors();
        this.apply_theme_ui(theme);
        this.sync_layout();
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
        self.action("new-calculation", &["<Control><Shift>n"], |w| w.new_calculation());
        self.action("new-presentation", &[], |w| w.new_presentation());
        self.action("present", &["<Control>F5"], |w| w.present(false, presenter::Layout::Fullscreen));
        self.action("present-current", &["<Shift>F5"], |w| w.present(true, presenter::Layout::Fullscreen));
        self.action("present-window", &["<Control><Shift>F5"], |w| w.present(true, presenter::Layout::Window));
        self.action("syntax-help", &["<Shift>F1"], |w| w.show_syntax_help());
        self.action("help", &["F1"], |w| {
            crate::help::show(&w.win, w.shared.theme.get(), w.shared.zoom.get());
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
                    w.toast(t!("Path copied"));
                }
                None => w.toast(t!("This note has no file")),
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
        // Dialogs (e.g. the preferences' search) keep their own Ctrl+F.
        self.action("find", &["<Control>f"], |w| {
            if w.win.visible_dialog().is_none() {
                w.open_search(false);
            }
        });
        self.action("replace", &["<Control>h"], |w| {
            if w.win.visible_dialog().is_none() {
                w.open_search(true);
            }
        });
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
        self.action("preferences", &["<Control>comma"], |w| w.show_preferences());
        self.action("emmet-expand", &["<Control>e"], |w| {
            if let Some(t) = w.current_text() {
                if !t.emmet_expand(false) {
                    w.toast(t!("No Emmet abbreviation before the cursor"));
                }
            }
        });
        self.action("emmet-wrap", &["<Control><Shift>g"], |w| w.emmet_wrap_popover());
        self.action("table-insert", &[], |w| {
            match w.current_text() {
                Some(t) if t.tasks_enabled() => t.insert_table(),
                Some(_) => w.toast(t!("Tables work in notes, Markdown and presentations")),
                None => {}
            }
        });
        for (name, accels, op) in TABLE_ACTIONS {
            self.action(name, accels, move |w| {
                if let Some(t) = w.current_text() {
                    t.table_op(*op);
                }
            });
        }
        self.action("toggle-task", &["<Control>Return", "<Control>KP_Enter"], |w| {
            match w.current_text() {
                Some(t) if t.tasks_enabled() => t.toggle_tasks_in_selection(),
                Some(_) => w.toast(t!("Checklists work in notes, Markdown and presentations")),
                None => {}
            }
        });
        self.action("about", &[], |w| {
            let about = adw::AboutDialog::builder()
                .application_name("QuickNotes")
                .application_icon(APP_ID)
                .version(env!("CARGO_PKG_VERSION"))
                .comments(t!("A small tabbed notepad for text and code snippets.\nNotes are stored in ~/.local/share/quicknotes."))
                .license_type(gtk::License::MitX11)
                .build();
            about.add_legal_section(
                "Emmet",
                Some("Copyright (c) 2020 Sergey Chikuyonok"),
                gtk::License::MitX11,
                None,
            );
            for (_, name, copyright) in themes::SOURCES.iter().filter(|s| !s.2.is_empty()) {
                about.add_legal_section(name, Some(copyright), gtk::License::MitX11, None);
            }
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
        self.toggle_action("focus-mode", &["<Alt>Return"], |w, on| w.set_focus_mode(on));

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

        let gc = gio::SimpleAction::new("gc", Some(glib::VariantTy::STRING));
        let weak = self.weak();
        gc.connect_activate(move |_, param| {
            if let (Some(this), Some(id)) = (weak.upgrade(), param.and_then(|p| p.str())) {
                this.gc_tool(id);
            }
        });
        gc.set_enabled(false);
        self.win.add_action(&gc);

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
        // Remember the width of the tab sidebar.
        let weak = self.weak();
        self.content.connect_position_notify(move |paned| {
            let Some(this) = weak.upgrade() else { return };
            if this.tab_list.root.is_visible() && paned.width() > 0 {
                this.settings.borrow_mut().sidebar_width = paned.position();
                this.schedule_persist();
            }
        });

        // Escape leaves distraction-free mode (capture phase: the editor
        // consumes Escape otherwise). Dialogs keep their own Escape.
        let keys = gtk::EventControllerKey::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = self.weak();
        keys.connect_key_pressed(move |_, key, _, mods| {
            let Some(this) = weak.upgrade() else { return glib::Propagation::Proceed };
            if key == gdk::Key::Escape
                && mods.is_empty()
                && this.focus_mode.get()
                && this.win.visible_dialog().is_none()
            {
                this.set_focus_mode(false);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        self.win.add_controller(keys);

        let weak = self.weak();
        self.view.connect_selected_page_notify(move |_| {
            if let Some(this) = weak.upgrade() {
                this.sync_ui();
                this.tab_list.sync_selection();
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
            if let Some(this) = weak.upgrade() {
                this.set_menu_page(page);
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
            self.toast(t!("Could not read the dropped files"));
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
            self.toast(t!("Could not read the dropped text"));
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

    /// Page a tab context menu is open for (`None` once the menu closed).
    fn set_menu_page(self: &Rc<Self>, page: Option<&adw::TabPage>) {
        match page {
            Some(p) => *self.menu_page.borrow_mut() = Some(p.clone()),
            None => {
                // Clear after the menu action had a chance to run.
                let weak = self.weak();
                glib::idle_add_local_once(move || {
                    if let Some(this) = weak.upgrade() {
                        this.menu_page.borrow_mut().take();
                    }
                });
            }
        }
    }

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
            Tab::Text(t) => {
                self.connect_text_tab(t);
                t.set_focus_mode(self.focus_mode.get());
            }
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
                        if prop == "has-selection" || this.current_text().is_some_and(|t| t.buffer.has_selection()) {
                            this.schedule_counts();
                        }
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
        *t.message_hook.borrow_mut() = Some(Box::new(move |text: &str| {
            if let Some(this) = weak.upgrade() {
                this.toast(text);
            }
        }));
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
            .unwrap_or_else(|| t!("Note – stored automatically").into());
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

    /// A new calculation sheet (see `calc.rs`).
    fn new_calculation(self: &Rc<Self>) {
        let t = self.new_note();
        t.language_locked.set(true);
        t.set_language_id(Some("calc"));
        self.mark_dirty(&t.id);
        self.sync_ui();
    }

    /// A new presentation with an example of the syntax (see `slides.rs`).
    fn new_presentation(self: &Rc<Self>) {
        let t = self.new_note();
        t.language_locked.set(true);
        t.set_text_irreversible(slides::template());
        t.set_language_id(Some(slides::LANGUAGE));
        t.place_cursor(CursorTarget::Start);
        t.set_preview(true);
        self.mark_dirty(&t.id);
        self.refresh_title(&t.id);
        self.sync_ui();
    }

    /// Starts the presentation of the current tab.
    fn present(self: &Rc<Self>, from_cursor: bool, layout: presenter::Layout) {
        let Some(t) = self.current_text() else { return };
        let Some(deck) = t.deck() else {
            self.toast(t!("This tab is no presentation"));
            return;
        };
        if deck.slides.is_empty() {
            self.toast(t!("The presentation has no slides yet"));
            return;
        }
        if let Some(running) = self.presentation.borrow_mut().take() {
            running.end();
        }
        let start = if from_cursor { t.current_slide() } else { 0 };
        let Some(app) = self.win.application().and_downcast::<adw::Application>() else { return };
        let weak = self.weak();
        let presentation = Presentation::start(&app, &self.win, &deck, t.slides_base_uri(), start, layout, move || {
            if let Some(this) = weak.upgrade() {
                this.presentation.borrow_mut().take();
                this.win.present();
            }
        });
        *self.presentation.borrow_mut() = Some(presentation);
    }

    /// Removes a tab from bookkeeping and remembers it for t!("Reopen Closed Tab").
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
                        .title(t!("“{}” closed", t.title()))
                        .button_label(t!("Undo"))
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
            Some(&t!("Save changes to “{}”?", t.title())),
            Some(t!("The file has unsaved changes. They will be lost if you don't save them.")),
        );
        dialog.add_responses(&[("cancel", t!("_Cancel")), ("discard", t!("_Discard")), ("save", t!("_Save"))]);
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
                            this.toast(&t!("Save failed: {}", e));
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
            None => self.toast(t!("No closed tabs")),
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
                    self.toast(&t!("Cannot open {}: {}", path.display(), e));
                    None
                }
            };
        }

        if let (None, Some(p)) = (&content, &meta.path) {
            if !p.is_file() {
                self.toast(&t!("File not found: {}", p.display()));
                return None;
            }
        }

        let t = EditorTab::new(meta.id.clone(), self.shared.clone());
        *t.path.borrow_mut() = meta.path.clone();
        *t.custom_title.borrow_mut() = meta.title.clone();
        t.language_locked.set(meta.language_locked);
        t.split.set(meta.split);
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
                    self.toast(&t!("Cannot store note: {}", e));
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
            .title(t!("Back Up Notes"))
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
                Ok(()) => this.toast(&t!("Backup of {} saved to {}", tabs_count(count), tilde(&path))),
                Err(e) => this.toast(&t!("Backup failed: {}", e)),
            }
        });
    }

    fn restore_backup(self: &Rc<Self>) {
        let filter = gtk::FileFilter::new();
        filter.set_name(Some(t!("QuickNotes backups")));
        filter.add_pattern("*.tar.gz");
        filter.add_pattern("*.tgz");
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        let dialog = gtk::FileDialog::builder()
            .title(t!("Restore Backup"))
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
                    this.toast(&t!("Restore failed: {}", e));
                    return;
                }
            };
            let created = backup.manifest.created.replace('T', " ");
            let created = created.get(..16).unwrap_or(&created);
            let dialog = adw::AlertDialog::new(
                Some(t!("Restore Backup?")),
                Some(&t!(
                    "Backup from {} with {}.\n\nAdd: puts the tabs of the backup next to your current tabs.\n\nReplace: closes all current tabs and restores the backup including its settings. Your current notes are saved to an automatic backup first.",
                    created,
                    tabs_count(backup.session.tabs.len())
                )),
            );
            dialog.add_responses(&[("cancel", t!("_Cancel")), ("add", t!("_Add")), ("replace", t!("_Replace"))]);
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
            0 => t!("All tabs of the backup are already open").to_owned(),
            n => t!("Added {} from the backup", tabs_count(n)),
        };
        if missing > 0 {
            msg.push_str(&t!(" – {} not found", files_count(missing)));
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
            self.toast(&t!("Restore cancelled, the current notes could not be saved: {}", e));
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
        self.set_theme(&restored.theme);
        self.shared.emmet_tab.set(restored.emmet_tab);
        self.apply_font();
        self.apply_tab_colors();
        self.sync_layout();

        let (count, missing) = self.import_tabs(&backup, false);
        if self.view.n_pages() == 0 {
            self.new_note();
        } else {
            let idx = (backup.session.active as i32).clamp(0, self.view.n_pages() - 1);
            self.view.set_selected_page(&self.view.nth_page(idx));
        }
        self.persist_all();
        self.sync_ui();
        let mut msg = t!("Restored {}. Previous notes saved to {}", tabs_count(count), tilde(&safety));
        if missing > 0 {
            msg.push_str(&t!(" – {} not found", files_count(missing)));
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
            self.toast(&t!("Not a file: {}", path.display()));
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
                    self.toast(&t!("Cannot open {}: {}", path.display(), e));
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
            if t.preview_kind() == Some(PreviewKind::Slides) {
                t.set_wrap(true);
            }
            self.add_tab(Tab::Text(t.clone()), true);
            if t.preview_kind().is_some_and(PreviewKind::auto_preview) && !large {
                t.set_preview(true);
                self.sync_ui();
            }
            self.load_into(&t, path, CursorTarget::Start);
            if large {
                self.toast(t!("Large file – syntax highlighting and word wrap are off"));
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
                this.toast(&t!("Cannot open {}: {}", path.display(), e));
            }
            this.refresh_title(&t.id);
            if this.is_current(&t.id) {
                this.update_status();
            }
        });
    }

    fn open_dialog(self: &Rc<Self>) {
        let dialog = gtk::FileDialog::builder().title(t!("Open Files")).modal(true).build();
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
                    self.toast(&t!("Saved {}", p.display()));
                    self.mark_dirty(&t.id);
                }
                Err(e) => self.toast(&t!("Save failed: {}", e)),
            },
            None => self.save_as(),
        }
    }

    /// Exports the current Markdown/reStructuredText document.
    fn export(self: &Rc<Self>, format: ExportFormat) {
        let Some(t) = self.current_text() else { return };
        if let Some(deck) = t.deck() {
            self.export_presentation(&t, deck, format);
            return;
        }
        let Some(body) = t.document_html() else {
            self.toast(t!("Export works for Markdown and reStructuredText"));
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
            .title(t!("Export as {}", ext.to_uppercase()))
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
                    this.toast(t!("Exporting PDF…"));
                    let weak = this.weak();
                    let target = path.clone();
                    crate::export::pdf(document, base_uri, path, crate::export::Page::A4, move |result| {
                        if let Some(this) = weak.upgrade() {
                            this.export_done(&target, result);
                        }
                    });
                }
            }
        });
    }

    /// Exports a presentation as stand-alone HTML file (with keyboard
    /// navigation and animations) or as PDF with one slide per page.
    fn export_presentation(self: &Rc<Self>, t: &Rc<EditorTab>, deck: slides::Deck, format: ExportFormat) {
        let ext = match format {
            ExportFormat::Html => "html",
            ExportFormat::Pdf => "pdf",
        };
        let stem = t
            .path
            .borrow()
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().trim_end_matches(".md").trim_end_matches(".slides").to_owned())
            .unwrap_or_else(|| file_name_for(&if deck.title.is_empty() { t.title() } else { deck.title.clone() }));
        let dialog = gtk::FileDialog::builder()
            .title(t!("Export as {}", ext.to_uppercase()))
            .modal(true)
            .initial_name(format!("{stem}.{ext}"))
            .build();
        if let Some(dir) = t.base_dir() {
            dialog.set_initial_folder(Some(&gio::File::for_path(dir)));
        }
        let mode = match format {
            ExportFormat::Html => slides::Mode::Export,
            ExportFormat::Pdf => slides::Mode::Print,
        };
        let document = slides::page(mode, Some(&deck), slides::PageOptions::default());
        let base_uri = t.slides_base_uri();
        let page_mm = deck.page_mm();
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
                    this.toast(t!("Exporting PDF…"));
                    let weak = this.weak();
                    let target = path.clone();
                    let page = crate::export::Page::Custom { width_mm: page_mm.0, height_mm: page_mm.1 };
                    crate::export::pdf(document, base_uri, path, page, move |result| {
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
                    .title(markup_escape(&t!("Exported to {}", tilde(path))))
                    .button_label(t!("Open"))
                    .timeout(6)
                    .build();
                let file = gio::File::for_path(path);
                let win = self.win.clone();
                toast.connect_button_clicked(move |_| {
                    gtk::FileLauncher::new(Some(&file)).launch(Some(&win), None::<&gio::Cancellable>, |_| {});
                });
                self.toasts.add_toast(toast);
            }
            Err(e) => self.toast(&t!("Export failed: {}", e)),
        }
    }

    fn save_as(self: &Rc<Self>) {
        let Some(t) = self.current_text() else { return };
        let dialog = gtk::FileDialog::builder().title(t!("Save As")).modal(true).build();
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
                    this.toast(&t!("Saved {}", path.display()));
                    this.refresh_title(&t.id);
                    this.mark_dirty(&t.id);
                    this.sync_ui();
                }
                Err(e) => this.toast(&t!("Save failed: {}", e)),
            }
        });
    }

    fn reload(self: &Rc<Self>) {
        let Some(t) = self.current_text() else { return };
        let Some(path) = t.path.borrow().clone() else {
            self.toast(t!("This note has no file to reload"));
            return;
        };
        if !t.buffer.is_modified() {
            t.set_large(self.is_large(&path));
            self.load_into(&t, path, t.cursor_target());
            return;
        }
        let dialog = adw::AlertDialog::new(
            Some(t!("Discard Changes and Reload?")),
            Some(t!("Your unsaved changes to this file will be lost.")),
        );
        dialog.add_responses(&[("cancel", t!("_Cancel")), ("reload", t!("_Reload"))]);
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
            self.toast(t!("Only text tabs can be renamed"));
            return;
        };
        let entry = gtk::Entry::builder()
            .text(t.custom_title.borrow().clone().unwrap_or_else(|| t.title()))
            .activates_default(true)
            .build();
        let dialog = adw::AlertDialog::new(
            Some(t!("Rename Tab")),
            Some(t!("Leave empty to name the tab after its first line or file.")),
        );
        dialog.set_extra_child(Some(&entry));
        dialog.add_responses(&[("cancel", t!("_Cancel")), ("rename", t!("_Rename"))]);
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
                    self.toast(t!("Nothing to change"));
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

    // ===== geocaching tools ==============================================

    fn gc_tool(self: &Rc<Self>, id: &str) {
        let Some(tool) = geocaching::find(id) else { return };
        let Some(t) = self.current_text() else { return };
        let Some((start, end)) = t.buffer.selection_bounds() else {
            self.toast(t!("Select some text first"));
            return;
        };
        let input = t.buffer.text(&start, &end, true).to_string();
        let mark = t.buffer.create_mark(None, &end, true);
        if tool.fields.is_empty() {
            match tool.run(&input, &Default::default()) {
                Ok(out) => self.gc_insert(&t, &mark, &out),
                Err(e) => self.toast(&e),
            }
            t.buffer.delete_mark(&mark);
            return;
        }
        self.gc_dialog(tool, t, input, mark);
    }

    /// Options dialog with a live preview of the result.
    fn gc_dialog(self: &Rc<Self>, tool: &'static geocaching::Tool, t: Rc<EditorTab>, input: String, mark: gtk::TextMark) {
        let mut initial = self.settings.borrow().gc_options.get(tool.id).cloned().unwrap_or_default();
        for f in tool.fields {
            let v = f.value(&initial);
            initial.insert(f.key().to_owned(), v);
        }
        let values = Rc::new(RefCell::new(initial));
        let input = Rc::new(input);

        let dialog = adw::AlertDialog::new(Some(crate::i18n::tr(tool.label)), None);
        dialog.add_responses(&[("cancel", t!("_Cancel")), ("copy", t!("_Copy")), ("insert", t!("_Insert Below"))]);
        dialog.set_response_appearance("insert", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("insert"));
        dialog.set_close_response("cancel");

        let preview = gtk::Label::builder()
            .xalign(0.0)
            .yalign(0.0)
            .wrap(true)
            .wrap_mode(gtk::pango::WrapMode::WordChar)
            .selectable(true)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(10)
            .margin_end(10)
            .build();
        preview.add_css_class("monospace");
        let refresh: Rc<dyn Fn()> = {
            let (values, input, preview, dialog) = (values.clone(), input.clone(), preview.clone(), dialog.downgrade());
            Rc::new(move || {
                let result = tool.run(&input, &values.borrow());
                let ok = result.is_ok();
                preview.set_text(&result.unwrap_or_else(|e| e));
                if ok {
                    preview.remove_css_class("error");
                } else {
                    preview.add_css_class("error");
                }
                if let Some(d) = dialog.upgrade() {
                    d.set_response_enabled("copy", ok);
                    d.set_response_enabled("insert", ok);
                }
            })
        };

        let list = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).build();
        list.add_css_class("boxed-list");
        for field in tool.fields {
            let key = field.key().to_owned();
            let current = values.borrow().get(&key).cloned().unwrap_or_default();
            let (values, refresh) = (values.clone(), refresh.clone());
            let row: gtk::Widget = match field {
                Field::Choice { label, options, .. } => {
                    let names: Vec<&str> = options.iter().map(|o| crate::i18n::tr(o.1)).collect();
                    let row = adw::ComboRow::builder()
                        .title(crate::i18n::tr(label))
                        .use_markup(false)
                        .model(&gtk::StringList::new(&names))
                        .selected(options.iter().position(|o| o.0 == current).unwrap_or(0) as u32)
                        .build();
                    row.connect_selected_notify(move |r| {
                        if let Some(o) = options.get(r.selected() as usize) {
                            values.borrow_mut().insert(key.clone(), o.0.to_owned());
                            refresh();
                        }
                    });
                    row.upcast()
                }
                Field::Toggle { label, .. } => {
                    let row = adw::SwitchRow::builder().title(crate::i18n::tr(label)).use_markup(false).active(current == "true").build();
                    row.connect_active_notify(move |r| {
                        values.borrow_mut().insert(key.clone(), r.is_active().to_string());
                        refresh();
                    });
                    row.upcast()
                }
                Field::Number { label, min, max, .. } => {
                    let row = adw::SpinRow::with_range(*min as f64, *max as f64, 1.0);
                    row.set_title(crate::i18n::tr(label));
                    row.set_use_markup(false);
                    row.set_value(current.parse().unwrap_or(0.0));
                    row.connect_value_notify(move |r| {
                        values.borrow_mut().insert(key.clone(), (r.value() as i32).to_string());
                        refresh();
                    });
                    row.upcast()
                }
            };
            list.append(&row);
        }

        let heading = gtk::Label::builder().label(t!("Result")).xalign(0.0).build();
        heading.add_css_class("heading");
        let scrolled = gtk::ScrolledWindow::builder()
            .child(&preview)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_height(true)
            .max_content_height(260)
            .build();
        let frame = gtk::Frame::builder().child(&scrolled).build();
        let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
        content.set_width_request(420);
        content.append(&list);
        content.append(&heading);
        content.append(&frame);
        dialog.set_extra_child(Some(&content));
        refresh();

        let this = self.clone();
        glib::spawn_future_local(async move {
            let response = dialog.choose_future(&this.win).await;
            if response == "insert" || response == "copy" {
                let values = values.borrow().clone();
                match tool.run(&input, &values) {
                    Ok(out) if response == "copy" => {
                        this.win.clipboard().set_text(&out);
                        this.toast(t!("Result copied"));
                    }
                    Ok(out) => this.gc_insert(&t, &mark, &out),
                    Err(e) => this.toast(&e),
                }
                this.settings.borrow_mut().gc_options.insert(tool.id.to_owned(), values);
                this.schedule_persist();
            }
            t.buffer.delete_mark(&mark);
        });
    }

    /// Inserts `out` on a new line below the line where `mark` (the end of
    /// the selection) is, and selects it.
    fn gc_insert(&self, t: &EditorTab, mark: &gtk::TextMark, out: &str) {
        let buf = &t.buffer;
        let mut pos = buf.iter_at_mark(mark);
        let (text, skip) = if pos.starts_line() {
            (format!("{out}\n"), 0)
        } else {
            if !pos.ends_line() {
                pos.forward_to_line_end();
            }
            (format!("\n{out}"), 1)
        };
        let start = pos.offset() + skip;
        buf.begin_user_action();
        buf.insert(&mut pos, &text);
        buf.end_user_action();
        let s = buf.iter_at_offset(start);
        let e = buf.iter_at_offset(start + out.chars().count() as i32);
        buf.select_range(&s, &e);
        t.view.scroll_mark_onscreen(&buf.selection_bound());
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

    fn set_theme(self: &Rc<Self>, id: &str) {
        let theme = {
            let mut s = self.settings.borrow_mut();
            let theme = themes::find(id, s.dark);
            s.theme = theme.id.clone();
            s.dark = theme.dark;
            theme
        };
        self.shared.theme.set(theme);
        self.apply_theme_ui(theme);
        for t in self.text_tabs() {
            t.apply_theme();
        }
        self.schedule_persist();
    }

    /// Light/dark variant and colour overrides of the window for `theme`.
    fn apply_theme_ui(&self, theme: &Theme) {
        adw::StyleManager::default().set_color_scheme(if theme.dark {
            adw::ColorScheme::ForceDark
        } else {
            adw::ColorScheme::ForceLight
        });
        self.theme_css.load_from_string(&theme.ui_css());
    }

    /// Loads the tab colours (defaults or user-defined) into the tab CSS.
    fn apply_tab_colors(&self) {
        let settings = self.settings.borrow();
        let mut css = String::new();
        for style in TabStyle::COLORED {
            let class = style.css_class();
            let color = settings
                .tab_colors
                .get(style.key())
                .filter(|c| gdk::RGBA::parse(c.as_str()).is_ok())
                .map_or(style.default_color(), String::as_str);
            css.push_str(&format!(
                "tabbar tab.{class}, .tab-sidebar row.{class} {{ background-color: alpha({color}, 0.15); }}\n\
                 tabbar tab.{class}:selected {{ background-color: alpha({color}, 0.33); box-shadow: inset 0 -2px {color}; }}\n\
                 .tab-sidebar row.{class}:selected {{ background-color: alpha({color}, 0.33); box-shadow: inset 3px 0 {color}; }}\n"
            ));
        }
        self.tab_css.load_from_string(&css);
    }

    /// Tab bar or sidebar, depending on the settings and distraction-free mode.
    fn sync_layout(&self) {
        let vertical = self.settings.borrow().vertical_tabs;
        let focus = self.focus_mode.get();
        self.tab_bar.set_visible(!vertical);
        self.tab_list.root.set_visible(vertical && !focus && self.view.n_pages() > 0);
        self.toolbar.set_reveal_top_bars(!focus);
        self.toolbar.set_reveal_bottom_bars(!focus);
    }

    fn set_focus_mode(self: &Rc<Self>, on: bool) {
        if self.focus_mode.replace(on) == on {
            return;
        }
        self.set_toggle("focus-mode", true, on);
        if on {
            self.close_search();
            self.win.fullscreen();
        } else {
            self.win.unfullscreen();
        }
        for t in self.text_tabs() {
            t.set_focus_mode(on);
        }
        self.sync_layout();
        if on {
            self.toast(t!("Distraction-free mode – Alt+Enter or Esc to leave"));
        }
        if let Some(t) = self.current_text() {
            t.view.grab_focus();
        }
    }

    fn show_preferences(self: &Rc<Self>) {
        let colors = TabStyle::COLORED
            .iter()
            .map(|s| prefs::TabColor { key: s.key(), label: s.label(), default: s.default_color() })
            .collect();
        let mut settings = self.settings.borrow().clone();
        settings.theme = self.shared.theme.get().id.clone();
        prefs::show(&self.win, &settings, colors, Rc::new(PrefsBridge(self.weak())));
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
        self.sync_layout();

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
                let slides = t.preview_kind() == Some(PreviewKind::Slides);
                self.set_presentable(slides);
                let emmet = t.emmet_syntax();
                self.set_enabled("emmet-expand", emmet.is_some());
                self.set_enabled("emmet-wrap", emmet.is_some_and(|s| !s.stylesheet));
            }
            _ => {
                self.lang_drop.set_visible(false);
                self.set_toggle("preview", false, false);
                self.set_toggle("wrap", false, false);
                self.set_enabled("export-html", false);
                self.set_enabled("export-pdf", false);
                self.set_enabled("gc", false);
                self.set_enabled("emmet-expand", false);
                self.set_enabled("emmet-wrap", false);
                self.set_presentable(false);
            }
        }
        self.guard.set(false);
        self.sync_menus();

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

    /// Rebuilds the main menu and the editors' context menus for the current
    /// tab types, and tags each tab in the tab bar with its type.
    fn sync_menus(&self) {
        let current = self.current();
        let context = MenuContext {
            style: current.as_ref().map(TabStyle::of),
            has_file: matches!(&current, Some(Tab::Text(t)) if t.path.borrow().is_some()),
            emmet: matches!(&current, Some(Tab::Text(t)) if t.emmet_syntax().is_some()),
        };
        if self.menu_context.get() != Some(context) {
            self.menu_context.set(Some(context));
            fill_main_menu(&self.main_menu, context);
        }

        let tabs = self.tabs.borrow();
        let mut styles = self.tab_styles.borrow_mut();
        styles.retain(|id, _| tabs.contains_key(id));
        for (id, entry) in tabs.iter() {
            let Tab::Text(t) = &entry.tab else { continue };
            let key = (TabStyle::of(&entry.tab), t.emmet_syntax().is_some());
            if styles.get(id) != Some(&key) {
                t.view.set_extra_menu(Some(&editor_context_menu(key.0, key.1)));
                styles.insert(id.clone(), key);
            }
        }

        // AdwTabBar has no per-page styling API; its tab widgets expose the
        // page they show, so the type class is set on them directly.
        for widget in descendants(&self.tab_bar) {
            if widget.css_name() != "tab" {
                continue;
            }
            let has_page = widget
                .find_property("page")
                .is_some_and(|p| p.value_type() == adw::TabPage::static_type());
            if !has_page {
                continue;
            }
            let page = widget.property::<Option<adw::TabPage>>("page");
            let style = page
                .and_then(|p| tabs.values().find(|e| e.page == p).map(|e| TabStyle::of(&e.tab)));
            set_style_class(&widget, style);
        }
        for (row, page) in self.tab_list.rows() {
            let style = tabs.values().find(|e| e.page == page).map(|e| TabStyle::of(&e.tab));
            set_style_class(row.upcast_ref(), style);
        }
    }

    /// Popover at the cursor listing what can be written there.
    fn show_syntax_help(&self) {
        let Some(t) = self.current_text() else { return };
        if t.preview_kind() != Some(PreviewKind::Slides) {
            return;
        }
        let (title, rows) = crate::completion::cheat_sheet_at(t.buffer.upcast_ref());

        let grid = gtk::Grid::builder().row_spacing(6).column_spacing(16).build();
        for (row, (syntax, info)) in rows.iter().enumerate() {
            let syntax = gtk::Label::builder()
                .label(syntax)
                .xalign(0.0)
                .yalign(0.0)
                .wrap(true)
                .max_width_chars(34)
                .selectable(true)
                .build();
            syntax.add_css_class("monospace");
            let info = gtk::Label::builder().label(info).xalign(0.0).yalign(0.0).wrap(true).max_width_chars(42).build();
            info.add_css_class("dim-label");
            grid.attach(&syntax, 0, row as i32, 1, 1);
            grid.attach(&info, 1, row as i32, 1, 1);
        }
        let heading = gtk::Label::builder().label(&title).xalign(0.0).build();
        heading.add_css_class("heading");
        let hint = gtk::Label::builder()
            .label(t!("Ctrl+Space lists the options at the cursor"))
            .xalign(0.0)
            .build();
        hint.add_css_class("dim-label");
        hint.add_css_class("caption");
        let scrolled = gtk::ScrolledWindow::builder()
            .child(&grid)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_height(true)
            .max_content_height(420)
            .build();
        let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
        content.set_margin_top(6);
        content.set_margin_bottom(6);
        content.set_margin_start(6);
        content.set_margin_end(6);
        content.append(&heading);
        content.append(&scrolled);
        content.append(&hint);

        let popover = gtk::Popover::builder().child(&content).position(gtk::PositionType::Bottom).build();
        popover.set_parent(&t.view);
        let iter = t.buffer.iter_at_mark(&t.buffer.get_insert());
        let rect = t.view.iter_location(&iter);
        let (x, y) = t.view.buffer_to_window_coords(gtk::TextWindowType::Widget, rect.x(), rect.y());
        popover.set_pointing_to(Some(&gdk::Rectangle::new(x, y, 1, rect.height())));
        popover.connect_closed(|p| {
            let p = p.clone();
            glib::idle_add_local_once(move || p.unparent());
        });
        popover.popup();
    }

    /// Asks for an abbreviation to wrap the selection (or line) with.
    fn emmet_wrap_popover(self: &Rc<Self>) {
        let Some(t) = self.current_text() else { return };
        if t.emmet_syntax().is_none_or(|s| s.stylesheet) {
            return;
        }
        let entry = gtk::Entry::builder()
            .placeholder_text(t!("Abbreviation, e.g. ul>li*"))
            .width_chars(28)
            .build();
        let heading = gtk::Label::builder().label(t!("Wrap with Emmet Abbreviation")).xalign(0.0).build();
        heading.add_css_class("heading");
        let content = gtk::Box::new(gtk::Orientation::Vertical, 8);
        for m in [6, 6] {
            content.set_margin_top(m);
            content.set_margin_bottom(m);
            content.set_margin_start(m);
            content.set_margin_end(m);
        }
        content.append(&heading);
        content.append(&entry);

        let popover = gtk::Popover::builder().child(&content).position(gtk::PositionType::Bottom).build();
        popover.set_parent(&t.view);
        let iter = match t.buffer.selection_bounds() {
            Some((start, _)) => start,
            None => t.buffer.iter_at_mark(&t.buffer.get_insert()),
        };
        let rect = t.view.iter_location(&iter);
        let (x, y) = t.view.buffer_to_window_coords(gtk::TextWindowType::Widget, rect.x(), rect.y());
        popover.set_pointing_to(Some(&gdk::Rectangle::new(x, y, 1, rect.height())));
        popover.connect_closed(|p| {
            let p = p.clone();
            glib::idle_add_local_once(move || p.unparent());
        });

        let weak = self.weak();
        let tab = Rc::downgrade(&t);
        let pop = popover.downgrade();
        entry.connect_activate(move |entry| {
            let (Some(this), Some(t)) = (weak.upgrade(), tab.upgrade()) else { return };
            let abbreviation = entry.text();
            if abbreviation.trim().is_empty() {
                return;
            }
            if let Some(p) = pop.upgrade() {
                p.popdown();
            }
            if let Err(e) = t.emmet_wrap(abbreviation.trim()) {
                this.toast(&t!("Emmet: {}", e));
            }
            t.view.grab_focus();
        });
        popover.popup();
        entry.grab_focus();
    }

    fn set_presentable(&self, on: bool) {
        for name in ["present", "present-current", "present-window", "syntax-help"] {
            self.set_enabled(name, on);
        }
        self.present_btn.set_visible(on);
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

    /// Recounts words etc. shortly after the last change (debounced).
    fn schedule_counts(&self) {
        if self.counts_pending.replace(true) {
            return;
        }
        glib::timeout_add_local_once(Duration::from_millis(150), || {
            WINDOW.with(|w| {
                if let Some(this) = w.borrow().as_ref() {
                    this.counts_pending.set(false);
                    this.update_counts();
                }
            });
        });
    }

    fn update_counts(&self) {
        let Some(t) = self.current_text().filter(|t| !t.large.get()) else {
            self.status_counts.set_text("");
            return;
        };
        let buf = &t.buffer;
        let (selected, text) = match buf.selection_bounds() {
            Some((s, e)) => (true, buf.text(&s, &e, true)),
            None => (false, buf.text(&buf.start_iter(), &buf.end_iter(), true)),
        };
        let c = stats::count(&text);
        let mut label = t!("{} words  ·  {} characters  ·  {} paragraphs", c.words, c.chars, c.paragraphs);
        if selected {
            label = t!("Selection: {}", label);
        }
        if t.tasks_enabled() {
            let (done, total) = checklist::progress(&text);
            if total > 0 {
                label = format!("{label}  ·  {}", t!("✓ {} of {} done", done, total));
            }
        }
        self.status_counts.set_text(&label);
    }

    fn update_status(&self) {
        self.schedule_counts();
        match self.current() {
            Some(Tab::Text(t)) => {
                let mut info = vec![match t.path.borrow().as_ref() {
                    Some(p) => tilde(p),
                    None => t!("Note").into(),
                }];
                info.push(t!("{} lines", t.buffer.line_count()));
                if t.large.get() {
                    info.push(t!("large file, highlighting off").into());
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
        let mut text = t!("Ln {}, Col {}", iter.line() + 1, iter.line_offset() + 1);
        self.set_enabled("gc", buf.has_selection());
        let in_table = t.table_at_cursor().is_some();
        for (name, _, _) in TABLE_ACTIONS {
            self.set_enabled(name, in_table);
        }
        if t.preview_kind() == Some(PreviewKind::Slides) {
            let lines = slides::slide_lines(&t.text());
            if !lines.is_empty() {
                let index = slides::slide_at(&lines, iter.line().max(0) as usize);
                text = format!("{}  ·  {}", t!("Slide {} of {}", index + 1, lines.len()), text);
            }
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
            Ok(_) if count > 0 => self.toast(&t!("Replaced {} occurrences", count)),
            Ok(_) => self.toast(t!("Nothing replaced")),
            Err(e) => self.toast(&t!("Replace failed: {}", e)),
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
            t!("No matches").to_owned()
        } else {
            match t.buffer.selection_bounds().map(|(s, e)| ctx.occurrence_position(&s, &e)) {
                Some(pos) if pos > 0 => t!("{} of {}", pos, total),
                _ => t!("{} matches", total),
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

/// GtkDropDown ignores Enter in its search field. Make Enter pick the first
/// match of the search, or keep the current item when nothing was typed.
pub(crate) fn enter_picks_first_match(drop: &gtk::DropDown) {
    let Some(entry) = descendants(drop).into_iter().find_map(|w| w.downcast::<gtk::SearchEntry>().ok()) else {
        return;
    };
    let drop = drop.downgrade();
    entry.connect_activate(move |entry| {
        let Some(drop) = drop.upgrade() else { return };
        let query = entry.text().to_lowercase();
        if !query.is_empty() {
            if let Some(model) = drop.model() {
                let first = (0..model.n_items()).find(|&i| {
                    model
                        .item(i)
                        .and_downcast::<gtk::StringObject>()
                        .is_some_and(|s| s.string().to_lowercase().contains(&query))
                });
                if let Some(position) = first {
                    drop.set_selected(position);
                }
            }
        }
        if let Some(popover) = entry.ancestor(gtk::Popover::static_type()).and_downcast::<gtk::Popover>() {
            popover.popdown();
        }
    });
}

fn build_search_ui() -> SearchUi {
    let settings = sourceview5::SearchSettings::new();
    settings.set_wrap_around(true);

    let find = gtk::SearchEntry::builder()
        .placeholder_text(t!("Find"))
        .hexpand(true)
        .build();
    let replace = gtk::Entry::builder()
        .placeholder_text(t!("Replace with"))
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
    find_row.append(&toggle("Aa", t!("Match case"), "case-sensitive"));
    find_row.append(&toggle("W", t!("Whole words"), "at-word-boundaries"));
    find_row.append(&toggle(".*", t!("Regular expression"), "regex-enabled"));
    find_row.append(&count);
    find_row.append(&close);

    let replace_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    replace_row.append(&replace);
    replace_row.append(&named_button("replace", None, Some(t!("Replace")), t!("Replace current match (Enter)")));
    replace_row.append(&named_button("replace-all", None, Some(t!("Replace All")), t!("Replace all matches")));
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

/// Kind of tab, used for the tab colour and the context-dependent menus.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TabStyle {
    Note,
    Code,
    Document,
    Slides,
    Calc,
    Image,
}

impl TabStyle {
    const ALL: [Self; 6] = [Self::Note, Self::Code, Self::Document, Self::Slides, Self::Calc, Self::Image];

    fn of(tab: &Tab) -> Self {
        let Tab::Text(t) = tab else { return Self::Image };
        match t.preview_kind() {
            Some(PreviewKind::Slides) => Self::Slides,
            Some(kind) if kind.is_document() => Self::Document,
            _ if t.is_calc() => Self::Calc,
            _ if t.language_id().is_none() => Self::Note,
            _ => Self::Code,
        }
    }

    /// Types with their own colour (plain notes keep the default look).
    const COLORED: [Self; 5] = [Self::Code, Self::Document, Self::Slides, Self::Calc, Self::Image];

    /// Key in `Settings::tab_colors`.
    fn key(self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Code => "code",
            Self::Document => "document",
            Self::Slides => "slides",
            Self::Calc => "calc",
            Self::Image => "image",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Note => t!("Notes"),
            Self::Code => t!("Code"),
            Self::Document => t!("Documents (Markdown, reStructuredText, HTML, PlantUML)"),
            Self::Slides => t!("Presentations"),
            Self::Calc => t!("Calculations"),
            Self::Image => t!("Images"),
        }
    }

    fn default_color(self) -> &'static str {
        match self {
            Self::Note => "#9a9996",
            Self::Code => "#2190a4",
            Self::Document => "#3584e4",
            Self::Slides => "#9141ac",
            Self::Calc => "#33d17a",
            Self::Image => "#ff7800",
        }
    }

    fn css_class(self) -> &'static str {
        match self {
            Self::Note => "tab-note",
            Self::Code => "tab-code",
            Self::Document => "tab-document",
            Self::Slides => "tab-slides",
            Self::Calc => "tab-calc",
            Self::Image => "tab-image",
        }
    }

    fn is_text(self) -> bool {
        self != Self::Image
    }

    fn exportable(self) -> bool {
        matches!(self, Self::Document | Self::Slides)
    }
}

/// Tags a tab widget (tab bar or sidebar row) with the class of its type.
fn set_style_class(widget: &gtk::Widget, style: Option<TabStyle>) {
    for s in TabStyle::ALL {
        if Some(s) != style {
            widget.remove_css_class(s.css_class());
        }
    }
    if let Some(s) = style {
        widget.add_css_class(s.css_class());
    }
}

/// Applies the changes made in the preferences dialog.
struct PrefsBridge(Weak<Window>);

impl PrefsBridge {
    fn with(&self, f: impl FnOnce(&Rc<Window>)) {
        if let Some(w) = self.0.upgrade() {
            f(&w);
        }
    }
}

impl prefs::Handler for PrefsBridge {
    fn set_theme(&self, id: &str) {
        self.with(|w| w.set_theme(id));
    }

    fn set_vertical_tabs(&self, vertical: bool) {
        self.with(|w| {
            w.settings.borrow_mut().vertical_tabs = vertical;
            w.sync_layout();
            w.sync_menus();
            w.tab_list.sync_selection();
            w.schedule_persist();
        });
    }

    fn set_font_family(&self, family: &str) {
        self.with(|w| {
            if w.settings.borrow().font_family == family {
                return;
            }
            w.settings.borrow_mut().font_family = family.to_owned();
            w.apply_font();
            w.schedule_persist();
        });
    }

    fn set_font_size(&self, size: f64) {
        self.with(|w| {
            w.settings.borrow_mut().font_size = size.clamp(MIN_FONT, MAX_FONT);
            w.apply_font();
            w.schedule_persist();
        });
    }

    fn set_emmet_tab(&self, on: bool) {
        self.with(|w| {
            w.settings.borrow_mut().emmet_tab = on;
            w.shared.emmet_tab.set(on);
            w.schedule_persist();
        });
    }

    fn set_ligatures(&self, on: bool) {
        self.with(|w| {
            w.settings.borrow_mut().ligatures = on;
            w.apply_font();
            w.schedule_persist();
        });
    }

    fn set_language(&self, language: &str) {
        self.with(|w| {
            w.settings.borrow_mut().language = language.to_owned();
            w.schedule_persist();
        });
    }

    fn set_tab_color(&self, kind: &str, color: Option<String>) {
        self.with(|w| {
            {
                let mut s = w.settings.borrow_mut();
                match color {
                    Some(c) => s.tab_colors.insert(kind.to_owned(), c),
                    None => s.tab_colors.remove(kind),
                };
            }
            w.apply_tab_colors();
            w.schedule_persist();
        });
    }

    fn backup(&self) {
        self.with(|w| w.backup());
    }

    fn restore_backup(&self) {
        self.with(|w| w.restore_backup());
    }

    fn open_data_dir(&self) {
        self.with(|w| {
            let launcher = gtk::FileLauncher::new(Some(&gio::File::for_path(store::data_dir())));
            launcher.launch(Some(&w.win), None::<&gio::Cancellable>, |_| {});
        });
    }
}

/// What the main menu depends on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct MenuContext {
    style: Option<TabStyle>,
    has_file: bool,
    /// Emmet is available for the current tab's language.
    emmet: bool,
}

fn menu_section(items: &[(&str, &str)]) -> gio::Menu {
    let s = gio::Menu::new();
    for (label, action) in items {
        s.append(Some(label), Some(action));
    }
    s
}

fn export_section() -> gio::Menu {
    menu_section(&[(t!("Export as HTML…"), "win.export-html"), (t!("Export as PDF…"), "win.export-pdf")])
}

fn presentation_section() -> gio::Menu {
    menu_section(&[
        (t!("Syntax Help"), "win.syntax-help"),
        (t!("Start Presentation"), "win.present"),
        (t!("Present from Current Slide"), "win.present-current"),
        (t!("Present in a Window"), "win.present-window"),
    ])
}

/// Fills the main menu with the entries that apply to the current tab.
fn fill_main_menu(menu: &gio::Menu, context: MenuContext) {
    menu.remove_all();
    let text = context.style.is_some_and(TabStyle::is_text);

    let mut file: Vec<(&str, &str)> = vec![
        (t!("New Note"), "win.new-note"),
        (t!("New Calculation"), "win.new-calculation"),
        (t!("New Presentation"), "win.new-presentation"),
        (t!("Open File…"), "win.open"),
    ];
    if text {
        file.extend([(t!("Save"), "win.save"), (t!("Save As…"), "win.save-as")]);
    }
    if context.has_file {
        file.push((t!("Reload from Disk"), "win.reload"));
    }
    menu.append_section(None, &menu_section(&file));

    if let Some(style) = context.style {
        if style.exportable() {
            menu.append_section(None, &export_section());
        }
        if style == TabStyle::Slides {
            menu.append_section(None, &presentation_section());
        }
    }

    let mut edit: Vec<(&str, &str)> = Vec::new();
    if text {
        edit.extend([(t!("Find…"), "win.find"), (t!("Find and Replace…"), "win.replace")]);
    }
    if context.style.is_some() {
        edit.push((t!("Rename Tab…"), "win.rename"));
    }
    edit.push((t!("Reopen Closed Tab"), "win.reopen-closed"));
    menu.append_section(None, &menu_section(&edit));
    if context.emmet {
        menu.append_section(None, &emmet_section());
    }

    let mut view: Vec<(&str, &str)> = Vec::new();
    if text {
        view.push((t!("Word Wrap"), "win.wrap"));
    }
    view.push((t!("Distraction-Free Mode"), "win.focus-mode"));
    menu.append_section(None, &menu_section(&view));

    menu.append_section(
        None,
        &menu_section(&[
            (t!("Preferences"), "win.preferences"),
            (t!("Help"), "win.help"),
            (t!("About QuickNotes"), "win.about"),
        ]),
    );
}

fn emmet_section() -> gio::Menu {
    menu_section(&[
        (t!("Expand Emmet Abbreviation"), "win.emmet-expand"),
        (t!("Wrap with Emmet Abbreviation…"), "win.emmet-wrap"),
    ])
}

/// Extra entries for an editor's right-click menu.
fn editor_context_menu(style: TabStyle, emmet: bool) -> gio::Menu {
    let menu = gio::Menu::new();
    if emmet {
        menu.append_section(None, &emmet_section());
    }
    if matches!(style, TabStyle::Note | TabStyle::Document | TabStyle::Slides) {
        let section = menu_section(&[(t!("Toggle Checklist Item"), "win.toggle-task"), (t!("Insert Table"), "win.table-insert")]);
        section.append_submenu(Some(t!("Table")), &table_menu());
        menu.append_section(None, &section);
    }
    menu.append_section(None, &quick_tools());
    menu.append_section(None, &tool_submenus());
    if style.exportable() {
        menu.append_section(None, &export_section());
    }
    if style == TabStyle::Slides {
        menu.append_section(None, &presentation_section());
    }
    menu.append_section(None, &geocaching_menu());
    menu
}

/// Table commands: action name, accelerators and operation.
const TABLE_ACTIONS: &[(&str, &[&str], table::Op)] = &[
    ("table-format", &[], table::Op::Format),
    ("table-row-above", &[], table::Op::InsertRowAbove),
    ("table-row-below", &[], table::Op::InsertRowBelow),
    ("table-row-delete", &[], table::Op::DeleteRow),
    ("table-row-up", &["<Alt><Shift>Up"], table::Op::MoveRowUp),
    ("table-row-down", &["<Alt><Shift>Down"], table::Op::MoveRowDown),
    ("table-col-left", &[], table::Op::InsertColumnLeft),
    ("table-col-right", &[], table::Op::InsertColumnRight),
    ("table-col-delete", &[], table::Op::DeleteColumn),
    ("table-col-move-left", &["<Alt><Shift>Left"], table::Op::MoveColumnLeft),
    ("table-col-move-right", &["<Alt><Shift>Right"], table::Op::MoveColumnRight),
    ("table-sort-asc", &[], table::Op::Sort(true)),
    ("table-sort-desc", &[], table::Op::Sort(false)),
    ("table-align-left", &[], table::Op::Align(table::Align::Left)),
    ("table-align-center", &[], table::Op::Align(table::Align::Center)),
    ("table-align-right", &[], table::Op::Align(table::Align::Right)),
    ("table-align-none", &[], table::Op::Align(table::Align::None)),
];

/// "Table" submenu for the editor's context menu.
fn table_menu() -> gio::Menu {
    let menu = gio::Menu::new();
    menu.append_section(None, &menu_section(&[(t!("Format Table"), "win.table-format")]));
    menu.append_section(
        None,
        &menu_section(&[
            (t!("Insert Row Above"), "win.table-row-above"),
            (t!("Insert Row Below"), "win.table-row-below"),
            (t!("Move Row Up"), "win.table-row-up"),
            (t!("Move Row Down"), "win.table-row-down"),
            (t!("Delete Row"), "win.table-row-delete"),
        ]),
    );
    menu.append_section(
        None,
        &menu_section(&[
            (t!("Insert Column Left"), "win.table-col-left"),
            (t!("Insert Column Right"), "win.table-col-right"),
            (t!("Move Column Left"), "win.table-col-move-left"),
            (t!("Move Column Right"), "win.table-col-move-right"),
            (t!("Delete Column"), "win.table-col-delete"),
        ]),
    );
    menu.append_section(
        None,
        &menu_section(&[
            (t!("Sort Ascending by Column"), "win.table-sort-asc"),
            (t!("Sort Descending by Column"), "win.table-sort-desc"),
        ]),
    );
    menu.append_section(
        None,
        &menu_section(&[
            (t!("Align Column Left"), "win.table-align-left"),
            (t!("Align Column Center"), "win.table-align-center"),
            (t!("Align Column Right"), "win.table-align-right"),
            (t!("Remove Column Alignment"), "win.table-align-none"),
        ]),
    );
    menu
}

/// "Geocaching" submenu for the editor's context menu.
fn geocaching_menu() -> gio::Menu {
    let sub = gio::Menu::new();
    for section in geocaching::MENU {
        let s = gio::Menu::new();
        for tool in section.iter().filter_map(|id| geocaching::find(id)) {
            let name = crate::i18n::tr(tool.label);
            let label = if tool.fields.is_empty() { name.to_owned() } else { format!("{name}…") };
            s.append(Some(&label), Some(&format!("win.gc::{}", tool.id)));
        }
        sub.append_section(None, &s);
    }
    let menu = gio::Menu::new();
    menu.append_submenu(Some("Geocaching"), &sub);
    menu
}

fn tool_items(list: &[(&str, &str)]) -> gio::Menu {
    let m = gio::Menu::new();
    for (label, tool) in list {
        m.append(Some(label), Some(&format!("win.tool::{tool}")));
    }
    m
}

fn quick_tools() -> gio::Menu {
    tool_items(&[(t!("Beautify (auto-detect)"), "beautify"), (t!("Minify (auto-detect)"), "minify")])
}

fn tools_menu() -> gio::Menu {
    let menu = gio::Menu::new();
    menu.append_section(Some(t!("Applies to the selection or the whole tab")), &quick_tools());
    menu.append_section(None, &tool_submenus());
    menu
}

fn tool_submenus() -> gio::Menu {
    let items = tool_items;
    let more = gio::Menu::new();
    more.append_submenu(
        Some(t!("Format")),
        &items(&[
            (t!("Pretty JSON"), "json-pretty"),
            (t!("Minify JSON"), "json-minify"),
            (t!("Pretty MongoDB / JSON5"), "mongo-pretty"),
            (t!("Minify MongoDB / JSON5"), "mongo-minify"),
            (t!("Pretty XML"), "xml-pretty"),
            (t!("Minify XML"), "xml-minify"),
            (t!("Pretty YAML"), "yaml-pretty"),
            (t!("Pretty SQL"), "sql-pretty"),
        ]),
    );
    more.append_submenu(
        Some(t!("Convert")),
        &items(&[
            ("JSON → YAML", "json-to-yaml"),
            ("YAML → JSON", "yaml-to-json"),
            ("mongosh → Extended JSON", "mongo-to-ejson"),
            (t!("mongosh → Extended JSON (canonical)"), "mongo-to-ejson-canonical"),
            ("Extended JSON → mongosh", "ejson-to-mongo"),
            (t!("PHP array → Extended JSON"), "php-to-ejson"),
            (t!("PHP array → mongosh"), "php-to-mongo"),
            (t!("Extended JSON / mongosh → PHP array"), "to-php"),
        ]),
    );
    more.append_submenu(
        Some(t!("Lines")),
        &items(&[
            (t!("Sort"), "sort-lines"),
            (t!("Sort (case-insensitive)"), "sort-lines-ci"),
            (t!("Reverse"), "reverse-lines"),
            (t!("Remove Duplicates"), "unique-lines"),
            (t!("Remove Empty Lines"), "remove-empty-lines"),
            (t!("Trim Trailing Whitespace"), "trim-trailing"),
        ]),
    );
    more.append_submenu(
        Some(t!("Encode / Decode")),
        &items(&[
            (t!("Base64 Encode"), "base64-encode"),
            (t!("Base64 Decode"), "base64-decode"),
            (t!("URL Encode"), "url-encode"),
            (t!("URL Decode"), "url-decode"),
            (t!("JSON String Escape"), "json-escape"),
            (t!("JSON String Unescape"), "json-unescape"),
        ]),
    );
    more.append_submenu(Some(t!("Case")), &items(&[(t!("UPPERCASE"), "upper"), (t!("lowercase"), "lower")]));
    more
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
    if n == 1 { t!("{} tab", n) } else { t!("{} tabs", n) }
}

fn files_count(n: usize) -> String {
    if n == 1 { t!("{} file", n) } else { t!("{} files", n) }
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
