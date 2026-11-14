//! Main window: tab management, actions, menus, search bar, status bar and
//! session persistence.

use crate::editor::{CursorTarget, EditorTab, LockState, PreviewKind, Shared};
use crate::image_tab::{self, ImageTab};
use crate::library::{self, TRASH, UNGROUPED};
use crate::notes_tree::{self, NotesTree};
use crate::store::{self, Folder, Session, Settings, TabKind, TabMeta};
use crate::features::{self, Feature, Features};
use crate::geocaching::{self, Field};
use crate::presenter::{self, Presentation};
use crate::prefs::{self, MAX_FONT, MIN_FONT};
use crate::tab_list::TabList;
use crate::themes::{self, Theme};
use crate::{checklist, crypto, format, format_bar, github, html, kanban, langs, md_format, slides, stats, table, APP_ID};
use adw::prelude::*;
use gtk::{gdk, gio, glib};
use sourceview5::prelude::*;
use webkit6::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use base64::Engine;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use zeroize::Zeroizing;

const BASE_FONT: f64 = 12.0;

#[derive(Clone)]
pub enum Tab {
    Text(Rc<EditorTab>),
    Image(Rc<ImageTab>),
}

impl Tab {
    fn widget(&self) -> gtk::Widget {
        match self {
            Tab::Text(t) => t.frame.clone().upcast(),
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
    status_problems: gtk::Button,
    problem_errors: gtk::Label,
    problem_warnings: gtk::Label,
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
    /// Header button of the unit converter (calculation sheets).
    convert_btn: gtk::Button,
    /// Last choices of the image export (format, layout).
    image_export: Cell<(usize, usize)>,
    /// Header button with the developer tools.
    tools_btn: gtk::MenuButton,
    presentation: RefCell<Option<Rc<Presentation>>>,
    /// The notes sidebar (folder tree) and the split it sits in.
    notes_tree: Rc<NotesTree>,
    notes_paned: gtk::Paned,
    notes_btn: gtk::ToggleButton,
    /// Folders of the notes library.
    folders: RefCell<Vec<Folder>>,
    /// Closed notes and file links of the library.
    library: RefCell<Vec<TabMeta>>,
    /// Folders of open tabs.
    tab_folders: RefCell<HashMap<String, String>>,
    /// Collapsed special places (Ungrouped, Trash).
    special_collapsed: RefCell<HashSet<String>>,
    /// Tabs that go to the trash when they close.
    trash_on_close: RefCell<HashSet<String>>,
    /// The running study session of flash cards.
    study: RefCell<Option<Rc<crate::study::Study>>>,
    /// The learning overview window and its web view.
    cards_overview: RefCell<Option<(gtk::Window, webkit6::WebView)>>,
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
    tab_styles: RefCell<HashMap<String, (TabStyle, bool, bool)>>,
    /// The app password (wrapped data key); `None` when off.
    vault: RefCell<Option<crypto::Lock>>,
    /// The window closes for locking: nothing must be written any more.
    locking: Cell<bool>,
    /// Sealed contents of password-protected tabs that are not open (closed
    /// tabs for *Reopen*, tabs of a backup being restored).
    sealed_tabs: RefCell<HashMap<String, Vec<u8>>>,
    /// GitHub sync: a run is going on; the window closes after syncing;
    /// cancels a pending sign-in.
    sync_busy: Cell<bool>,
    /// The last automatic sync failed (its error was shown once).
    sync_failed: Cell<bool>,
    /// Stops a running Nextcloud sign-in.
    sync_login: RefCell<Option<Arc<AtomicBool>>>,
    sync_closing: Cell<bool>,
    sync_cancel: RefCell<Option<Arc<AtomicBool>>>,
    /// Notes deleted elsewhere whose tab closes without keeping them.
    sync_discard: RefCell<HashSet<String>>,
    /// Pending update of notes created from templates.
    template_timer: RefCell<Option<glib::SourceId>>,
}

thread_local! {
    static WINDOW: RefCell<Option<Rc<Window>>> = const { RefCell::new(None) };
}

/// Shows the window (creating and restoring it on first call) and opens `files`.
/// With the app password on, the unlock window comes first.
pub fn present(app: &adw::Application, files: &[gio::File]) {
    let paths: Vec<PathBuf> = files.iter().filter_map(|f| f.path()).collect();
    if let Some(w) = WINDOW.with(|w| w.borrow().clone()) {
        for path in paths {
            w.open_path(path);
        }
        w.win.present();
        return;
    }
    if UNLOCK.with(|u| u.borrow().is_some()) {
        UNLOCK.with(|u| {
            if let Some(unlock) = u.borrow_mut().as_mut() {
                unlock.files.extend(paths);
                unlock.win.present();
            }
        });
        return;
    }
    let session = store::load_session();
    if session.vault.is_some() && store::app_key().is_none() {
        show_unlock(app, session, paths);
    } else {
        open_main(app, session, paths);
    }
}

fn open_main(app: &adw::Application, session: Session, files: Vec<PathBuf>) {
    let w = Window::new(app, session.settings.clone());
    WINDOW.with(|c| *c.borrow_mut() = Some(w.clone()));
    *w.vault.borrow_mut() = session.vault.clone();
    // Notes from before: an update rather than a new installation.
    let existing = !session.tabs.is_empty() || !session.library.is_empty() || session.sealed.is_some();
    w.restore(session);
    for path in files {
        w.open_path(path);
    }
    w.win.present();
    w.show_whats_new(existing);
    w.check_for_updates();
    w.github_startup();
    w.sync_timer();
}

/// The unlock window shown instead of the notes while the app password is on.
struct Unlock {
    win: adw::ApplicationWindow,
    /// Files to open once unlocked.
    files: Vec<PathBuf>,
}

thread_local! {
    static UNLOCK: RefCell<Option<Unlock>> = const { RefCell::new(None) };
}

fn show_unlock(app: &adw::Application, session: Session, files: Vec<PathBuf>) {
    crate::i18n::init(&session.settings.language);
    let scheme = if session.settings.dark { adw::ColorScheme::ForceDark } else { adw::ColorScheme::ForceLight };
    adw::StyleManager::default().set_color_scheme(scheme);
    let win = adw::ApplicationWindow::builder()
        .application(app)
        .title("QuickNotes")
        .default_width(480)
        .default_height(560)
        .build();
    let entry = gtk::PasswordEntry::builder()
        .show_peek_icon(true)
        .placeholder_text(t!("App password"))
        .build();
    let button = gtk::Button::builder().label(t!("Unlock")).halign(gtk::Align::Center).build();
    button.add_css_class("suggested-action");
    button.add_css_class("pill");
    let error = gtk::Label::new(None);
    error.add_css_class("error");
    let spinner = gtk::Spinner::new();
    let column = gtk::Box::new(gtk::Orientation::Vertical, 12);
    column.set_width_request(300);
    column.set_halign(gtk::Align::Center);
    error.set_wrap(true);
    error.set_justify(gtk::Justification::Center);
    for w in [entry.upcast_ref::<gtk::Widget>(), error.upcast_ref(), button.upcast_ref(), spinner.upcast_ref()] {
        column.append(w);
    }
    let page = adw::StatusPage::builder()
        .icon_name("system-lock-screen-symbolic")
        .title(t!("QuickNotes Is Locked"))
        .description(t!("Enter the app password to open your notes."))
        .child(&column)
        .build();
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&page));
    win.set_content(Some(&toolbar));
    win.connect_close_request(|_| {
        UNLOCK.with(|u| u.borrow_mut().take());
        glib::Propagation::Proceed
    });
    UNLOCK.with(|u| *u.borrow_mut() = Some(Unlock { win: win.clone(), files }));

    let session = Rc::new(RefCell::new(Some(session)));
    let attempt = {
        let (app, win, entry, button, error, spinner) =
            (app.clone(), win.clone(), entry.clone(), button.clone(), error.clone(), spinner.clone());
        Rc::new(move || {
            let password = Zeroizing::new(entry.text().to_string());
            let Some(lock) = session.borrow().as_ref().and_then(|s| s.vault.clone()) else { return };
            if password.is_empty() {
                return;
            }
            entry.set_sensitive(false);
            button.set_sensitive(false);
            spinner.start();
            error.set_text("");
            let (app, win, entry, button, error, spinner, session) =
                (app.clone(), win.clone(), entry.clone(), button.clone(), error.clone(), spinner.clone(), session.clone());
            glib::spawn_future_local(async move {
                let result = gio::spawn_blocking(move || lock.open(&password)).await;
                spinner.stop();
                let opened = match result {
                    Ok(Ok(key)) => {
                        let mut s = session.borrow().clone().unwrap_or_default();
                        s.unseal(&key).map(|()| (s, key))
                    }
                    Ok(Err(e)) => Err(e),
                    Err(_) => Err(crypto::Error::Format),
                };
                match opened {
                    Ok((s, key)) => {
                        entry.set_text("");
                        store::set_app_key(Some(key));
                        let files = UNLOCK.with(|u| u.borrow_mut().take()).map(|u| u.files).unwrap_or_default();
                        open_main(&app, s, files);
                        win.destroy();
                    }
                    Err(e) => {
                        error.set_text(&e.to_string());
                        entry.set_sensitive(true);
                        button.set_sensitive(true);
                        entry.select_region(0, -1);
                        entry.grab_focus();
                    }
                }
            });
        })
    };
    let a = attempt.clone();
    entry.connect_activate(move |_| a());
    button.connect_clicked(move |_| attempt());
    win.present();
    entry.grab_focus();
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
        features::apply(settings.features);
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
        let notes_btn = gtk::ToggleButton::builder()
            .icon_name("sidebar-show-symbolic")
            .tooltip_text(t!("Notes and folders (F9)"))
            .action_name("win.notes-sidebar")
            .visible(features::on(Feature::Folders))
            .build();
        header.pack_start(&notes_btn);
        header.pack_start(&icon_button("tab-new-symbolic", t!("New note (Ctrl+N)"), "win.new-note"));
        header.pack_start(&icon_button("document-open-symbolic", t!("Open file (Ctrl+O)"), "win.open"));
        header.pack_start(&icon_button("document-save-symbolic", t!("Save to file (Ctrl+S)"), "win.save"));

        let main_menu = gio::Menu::new();
        fill_main_menu(&main_menu, MenuContext { style: None, has_file: false, emmet: false, app_password: false, features: features::current(), lock: None, data: false, linked: false });
        let menu_btn = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .popover(&nested_menu(&main_menu))
            .primary(true)
            .tooltip_text(t!("Main menu"))
            .build();
        let tools_btn = gtk::MenuButton::builder()
            .icon_name("format-justify-left-symbolic")
            .popover(&nested_menu(&tools_menu()))
            .tooltip_text(t!("Developer Tools"))
            .visible(features::on(Feature::DevTools))
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
            .popover(&nested_menu(&present_menu))
            .visible(false)
            .build();
        header.pack_end(&menu_btn);
        header.pack_end(&tools_btn);
        header.pack_end(&preview_btn);
        header.pack_end(&find_btn);
        header.pack_end(&present_btn);
        let convert_btn = icon_button(&crate::icons::format_icon("convert"), t!("Unit Converter"), "win.unit-converter");
        convert_btn.set_visible(false);
        header.pack_end(&convert_btn);

        // ----- tabs ------------------------------------------------------
        let view = adw::TabView::new();
        let tab_menu = gio::Menu::new();
        tab_menu.append(Some(t!("Rename…")), Some("win.tab-rename"));
        tab_menu.append(Some(t!("Copy Path")), Some("win.tab-copy-path"));
        tab_menu.append_section(None, &lock_section(true));
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
        // Problems of JSON, YAML and XML documents; toggles the problems list.
        let problem_errors = gtk::Label::new(None);
        let problem_warnings = gtk::Label::new(None);
        let problems_content = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        let error_icon = gtk::Image::from_icon_name("dialog-error-symbolic");
        let warning_icon = gtk::Image::from_icon_name("dialog-warning-symbolic");
        warning_icon.set_margin_start(6);
        problems_content.append(&error_icon);
        problems_content.append(&problem_errors);
        problems_content.append(&warning_icon);
        problems_content.append(&problem_warnings);
        let status_problems = gtk::Button::builder().child(&problems_content).action_name("win.problems").visible(false).build();
        status_problems.add_css_class("flat");
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
        status.append(&status_problems);
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

        // The notes sidebar with the folder tree, left of everything.
        let tree_target: Rc<RefCell<Option<Weak<Window>>>> = Rc::default();
        let target = tree_target.clone();
        let notes_tree = NotesTree::new(move |event| {
            if let Some(this) = target.borrow().as_ref().and_then(Weak::upgrade) {
                this.on_tree_event(event);
            }
        });
        let notes_paned = gtk::Paned::new(gtk::Orientation::Horizontal);
        notes_paned.set_start_child(Some(&notes_tree.root));
        notes_paned.set_end_child(Some(&content));
        notes_paned.set_resize_start_child(false);
        notes_paned.set_shrink_start_child(false);
        notes_paned.set_shrink_end_child(false);
        notes_paned.set_position(settings.notes_sidebar_width.max(140));

        let toasts = adw::ToastOverlay::new();
        toasts.set_child(Some(&notes_paned));

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
            format_bar: Cell::new(settings.format_bar),
            learning: RefCell::new(crate::learning::Store::load()),
            problems_panel: Cell::new(settings.problems_panel),
            yaml_rules: Cell::new(settings.yaml_rules),
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
            status_problems,
            problem_errors,
            problem_warnings,
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
            convert_btn,
            image_export: Cell::new((0, 0)),
            tools_btn: tools_btn.clone(),
            presentation: RefCell::new(None),
            notes_tree,
            notes_paned,
            notes_btn,
            folders: RefCell::new(Vec::new()),
            library: RefCell::new(Vec::new()),
            tab_folders: RefCell::new(HashMap::new()),
            special_collapsed: RefCell::new(HashSet::new()),
            trash_on_close: RefCell::new(HashSet::new()),
            study: RefCell::new(None),
            cards_overview: RefCell::new(None),
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
            vault: RefCell::new(None),
            locking: Cell::new(false),
            sealed_tabs: RefCell::new(HashMap::new()),
            sync_busy: Cell::new(false),
            sync_failed: Cell::new(false),
            sync_login: RefCell::new(None),
            sync_closing: Cell::new(false),
            sync_cancel: RefCell::new(None),
            sync_discard: RefCell::new(HashSet::new()),
            template_timer: RefCell::new(None),
        });

        *menu_target.borrow_mut() = Some(this.weak());
        *tree_target.borrow_mut() = Some(this.weak());
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

    /// Markdown formatting commands (toolbar and shortcuts).
    fn set_format_enabled(&self, enabled: bool) {
        for (name, _, _) in FORMAT_ACTIONS {
            self.set_enabled(name, enabled);
        }
        for (label, _) in md_format::COLORS {
            self.set_enabled(&format_bar::color_action(label), enabled);
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
        self.action("unit-converter", &[], |w| w.unit_converter());
        self.action("import-obsidian", &[], |w| w.import_obsidian());
        self.action("new-presentation", &[], |w| {
            if features::on(Feature::Presentations) {
                w.new_presentation();
            }
        });
        self.action("github-sync", &[], |w| w.github_sync_now());
        self.action("new-kanban", &["<Control><Shift>k"], |w| {
            if features::on(Feature::Kanban) {
                w.new_kanban();
            }
        });
        self.toggle_action("notes-sidebar", &["F9"], |w, on| {
            w.settings.borrow_mut().notes_sidebar = on;
            w.sync_layout();
            w.schedule_persist();
        });
        type LibAction = fn(&Rc<Window>, String);
        let lib_actions: [(&str, LibAction); 11] = [
            ("lib-open", |w, id| w.open_note(&id)),
            ("lib-new-folder", |w, parent| w.new_folder(parent)),
            ("lib-rename-folder", |w, id| w.rename_folder(id)),
            ("lib-delete-folder", |w, id| w.delete_folder(&id)),
            ("lib-new-note", |w, folder| {
                let t = w.new_note();
                if !folder.is_empty() {
                    w.tab_folders.borrow_mut().insert(t.id.clone(), folder);
                    w.refresh_tree();
                }
            }),
            ("lib-trash", |w, id| w.trash_note(&id)),
            ("lib-restore", |w, id| w.restore_note(&id)),
            ("lib-delete", |w, id| w.delete_notes(vec![id])),
            ("lib-empty-trash", |w, _| {
                let ids = w.library.borrow().iter().filter(|m| m.trashed).map(|m| m.id.clone()).collect();
                w.delete_notes(ids);
            }),
            ("lib-remove", |w, id| w.remove_link(&id)),
            ("lib-ungroup", |w, id| {
                w.set_note_folder(&id, None);
                w.refresh_tree();
                w.schedule_persist();
            }),
        ];
        for (name, f) in lib_actions {
            let action = gio::SimpleAction::new(name, Some(glib::VariantTy::STRING));
            let weak = self.weak();
            action.connect_activate(move |_, param| {
                let (Some(this), Some(arg)) = (weak.upgrade(), param.and_then(|p| p.str())) else { return };
                if features::on(Feature::Folders) {
                    f(&this, arg.to_owned());
                }
            });
            self.win.add_action(&action);
        }
        self.action("new-flashcards", &[], |w| {
            if features::on(Feature::Flashcards) {
                w.new_flashcards();
            }
        });
        self.action("cards-overview", &[], |w| {
            if features::on(Feature::Flashcards) {
                w.show_cards_overview();
            }
        });
        let learn = gio::SimpleAction::new("cards-learn", Some(glib::VariantTy::STRING));
        let weak = self.weak();
        learn.connect_activate(move |_, param| {
            let (Some(this), Some(target)) = (weak.upgrade(), param.and_then(|p| p.str())) else { return };
            let (mode, deck) = target.split_once('\t').unwrap_or(("learn", target));
            match this.current_text() {
                Some(t) if t.preview_kind() == Some(PreviewKind::Flashcards) => this.study(vec![t], deck, mode == "practice"),
                _ => {}
            }
        });
        self.win.add_action(&learn);
        self.action("lock-app", &[], |w| w.lock_app());
        self.action("tab-protect", &[], |w| {
            if let Some(t) = w.menu_text_tab() {
                w.protect_tab(t);
            }
        });
        self.action("tab-lock", &[], |w| {
            if let Some(t) = w.menu_text_tab() {
                t.lock_now();
            }
        });
        self.action("tab-change-password", &[], |w| {
            if let Some(t) = w.menu_text_tab() {
                w.change_tab_password(t);
            }
        });
        self.action("tab-remove-password", &[], |w| {
            if let Some(t) = w.menu_text_tab() {
                w.remove_tab_password(t);
            }
        });
        self.action("present", &["<Control>F5"], |w| w.present(false, presenter::Layout::Fullscreen));
        self.action("present-current", &["<Shift>F5"], |w| w.present(true, presenter::Layout::Fullscreen));
        self.action("present-window", &["<Control><Shift>F5"], |w| w.present(true, presenter::Layout::Window));
        self.action("syntax-help", &["<Shift>F1"], |w| w.show_syntax_help());
        self.action("help", &["F1"], |w| {
            let topic = w.help_topic();
            crate::help::show(&w.win, w.shared.theme.get(), w.shared.zoom.get(), topic);
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
        self.action("export-image", &[], |w| w.export_image());
        self.action("export-epub", &[], |w| w.export_epub());
        self.action("export-svg", &[], |w| w.export(ExportFormat::Svg));
        self.action("new-mindmap", &[], |w| {
            if features::on(Feature::Mindmaps) {
                w.new_mindmap();
            }
        });
        self.action("new-adventure", &[], |w| {
            if features::on(Feature::GmTools) {
                w.new_adventure();
            }
        });
        let character = gio::SimpleAction::new("insert-character", Some(glib::VariantTy::STRING));
        let weak = self.weak();
        character.connect_activate(move |_, param| {
            let (Some(this), Some(id)) = (weak.upgrade(), param.and_then(|p| p.str())) else { return };
            let Some(t) = this.current_text() else { return };
            if let Some((_, _, sheet)) = crate::adventure::presets().into_iter().find(|(p, _, _)| *p == id) {
                t.insert_block(sheet, 1);
            }
        });
        self.win.add_action(&character);
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
        self.action("table-import", &[], |w| {
            match w.current_text() {
                Some(t) if t.tasks_enabled() => w.import_data(t),
                Some(_) => w.toast(t!("Tables work in notes, Markdown and presentations")),
                None => {}
            }
        });
        self.action("table-paste", &[], |w| {
            match w.current_text() {
                Some(t) if t.tasks_enabled() => w.paste_table(t),
                Some(_) => w.toast(t!("Tables work in notes, Markdown and presentations")),
                None => {}
            }
        });
        let chart = gio::SimpleAction::new("chart-insert", Some(glib::VariantTy::STRING));
        let weak = self.weak();
        chart.connect_activate(move |_, param| {
            let (Some(this), Some(kind)) = (weak.upgrade(), param.and_then(|p| p.str())) else { return };
            if !features::on(Feature::Charts) {
                return;
            }
            match this.current_text() {
                Some(t) if t.tasks_enabled() => t.insert_chart(kind),
                Some(_) => this.toast(t!("Charts work in notes, Markdown and presentations")),
                None => {}
            }
        });
        self.win.add_action(&chart);
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
                .release_notes_version(env!("CARGO_PKG_VERSION"))
                .release_notes(crate::changelog::release_notes())
                .comments(t!("A small tabbed notepad for text and code snippets.\nNotes are stored in ~/.local/share/quicknotes."))
                .license_type(gtk::License::MitX11)
                .build();
            about.add_legal_section(
                "Emmet",
                Some("Copyright (c) 2020 Sergey Chikuyonok"),
                gtk::License::MitX11,
                None,
            );
            about.add_legal_section(
                "KaTeX",
                Some("Copyright (c) 2013-2020 Khan Academy and other contributors"),
                gtk::License::MitX11,
                None,
            );
            about.add_legal_section(
                "abcjs",
                Some("Copyright (c) 2009-2026 Paul Rosen and Gregory Dyke"),
                gtk::License::MitX11,
                None,
            );
            about.add_legal_section(
                "Apache ECharts",
                Some("Copyright 2017-2026 The Apache Software Foundation"),
                gtk::License::Apache20,
                None,
            );
            about.add_legal_section("calamine", Some("Copyright (c) 2016 Johann Tuffe"), gtk::License::MitX11, None);
            about.add_legal_section("rust-csv", Some("Copyright (c) 2015 Andrew Gallant"), gtk::License::MitX11, None);
            about.add_legal_section("hayro", Some("Copyright (c) The Hayro Authors"), gtk::License::MitX11, None);
            about.add_legal_section("jpeg-encoder", Some("Copyright (c) 2021 Volker Ströbel"), gtk::License::MitX11, None);
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
        self.toggle_action("problems", &["<Control><Shift>m"], |w, on| {
            w.settings.borrow_mut().problems_panel = on;
            w.shared.problems_panel.set(on);
            for t in w.text_tabs() {
                t.sync_problems_panel();
            }
            w.schedule_persist();
        });
        let problems = self.settings.borrow().problems_panel;
        self.set_toggle("problems", true, problems);
        self.action("assign-schema", &[], |w| w.assign_schema());
        self.action("new-from-template", &["<Control><Alt>n"], |w| w.pick_template(TemplateUse::New));
        self.action("insert-template", &["<Control><Alt>i"], |w| {
            if w.current_text().is_some() {
                w.pick_template(TemplateUse::Insert);
            }
        });
        self.action("edit-templates", &[], |w| w.pick_template(TemplateUse::Edit));
        self.action("detach-template", &[], |w| {
            if let Some(t) = w.current_text() {
                if let Some(link) = t.template.borrow_mut().take() {
                    w.toast(&t!("No longer updated from template “{}”", link.name));
                }
                w.mark_dirty(&t.id);
                w.sync_ui();
            }
        });
        self.action("new-template", &[], |w| w.new_template(None));
        self.action("save-as-template", &[], |w| {
            if let Some(t) = w.current_text() {
                w.new_template(Some(t));
            }
        });
        // Downloaded schemas: check the documents again.
        let weak = self.weak();
        crate::schemas::set_listener(move || {
            if let Some(this) = weak.upgrade() {
                for t in this.text_tabs() {
                    if t.data_kind().is_some() {
                        t.lint();
                    }
                }
            }
        });
        self.toggle_action("format-bar", &[], |w, on| {
            w.settings.borrow_mut().format_bar = on;
            w.set_format_bar(on);
            w.schedule_persist();
        });
        let format_bar = self.settings.borrow().format_bar;
        self.set_toggle("format-bar", true, format_bar);
        let notes_sidebar = self.settings.borrow().notes_sidebar;
        self.set_toggle("notes-sidebar", true, notes_sidebar);
        for (name, accels, op) in FORMAT_ACTIONS {
            self.action(name, accels, move |w| {
                if let Some(t) = w.current_text() {
                    t.md_format(*op);
                }
            });
        }
        for (label, color) in md_format::COLORS {
            self.action(&format_bar::color_action(label), &[], move |w| {
                if let Some(t) = w.current_text() {
                    t.md_format(md_format::Op::Color(Some(color)));
                }
            });
        }

        let tool = gio::SimpleAction::new("tool", Some(glib::VariantTy::STRING));
        let weak = self.weak();
        tool.connect_activate(move |_, param| {
            if let (Some(this), Some(name)) = (weak.upgrade(), param.and_then(|p| p.str())) {
                // Without the developer tools, Beautify still formats Markdown tables.
                let tables = name == "beautify"
                    && this.current_text().is_some_and(|t| {
                        matches!(t.preview_kind(), Some(PreviewKind::Markdown | PreviewKind::Slides))
                    });
                if features::on(Feature::DevTools) || tables {
                    this.apply_tool(name);
                }
            }
        });
        self.win.add_action(&tool);
        app.set_accels_for_action("win.tool::beautify", &["<Control><Alt>b"]);
        app.set_accels_for_action("win.tool::minify", &["<Control><Alt>m"]);

        let gc = gio::SimpleAction::new("gc", Some(glib::VariantTy::STRING));
        let weak = self.weak();
        gc.connect_activate(move |_, param| {
            if let (Some(this), Some(id)) = (weak.upgrade(), param.and_then(|p| p.str())) {
                if features::on(Feature::Geocaching) {
                    this.gc_tool(id);
                }
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
        let weak = self.weak();
        self.notes_paned.connect_position_notify(move |paned| {
            let Some(this) = weak.upgrade() else { return };
            if this.notes_tree.root.is_visible() && paned.width() > 0 {
                this.settings.borrow_mut().notes_sidebar_width = paned.position();
                this.schedule_persist();
            }
        });
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
                if !this.locking.get() {
                    this.persist_all();
                    if this.github_ready() && !this.sync_closing.replace(true) {
                        this.close_with_sync();
                        return glib::Propagation::Stop;
                    }
                }
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
            Some(p) => {
                *self.menu_page.borrow_mut() = Some(p.clone());
                self.update_lock_actions();
            }
            None => {
                // Clear after the menu action had a chance to run.
                let weak = self.weak();
                glib::idle_add_local_once(move || {
                    if let Some(this) = weak.upgrade() {
                        this.menu_page.borrow_mut().take();
                        this.update_lock_actions();
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

    /// Shows or hides the formatting toolbar of all Markdown tabs.
    fn set_format_bar(&self, on: bool) {
        self.shared.format_bar.set(on);
        self.set_toggle("format-bar", true, on);
        for t in self.text_tabs() {
            t.sync_format_bar();
        }
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
        *t.lock_hook.borrow_mut() = Some(Box::new(move || {
            let (Some(this), Some(t)) = (weak.upgrade(), tab.upgrade()) else { return };
            this.update_lock_icon(&t);
            this.refresh_title(&t.id);
            this.mark_dirty(&t.id);
            this.sync_ui();
        }));

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
            if this.tab_folders.borrow().get(&t.id).is_some_and(|f| f == TEMPLATES_FOLDER) {
                this.schedule_template_refresh();
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
        let tab_id = id.clone();
        *t.lint_hook.borrow_mut() = Some(Box::new(move || {
            if let Some(this) = weak.upgrade() {
                if this.is_current(&tab_id) {
                    this.update_problems_status();
                }
            }
        }));
        let weak = self.weak();
        *t.message_hook.borrow_mut() = Some(Box::new(move |text: &str| {
            if let Some(this) = weak.upgrade() {
                this.toast(text);
            }
        }));
        let weak = self.weak();
        *t.note_link_hook.borrow_mut() = Some(Box::new(move |title: &str| {
            if let Some(this) = weak.upgrade() {
                this.open_note_titled(title);
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
        let mut tooltip = entry
            .tab
            .path()
            .map(|p| markup_escape(&p.to_string_lossy()))
            .unwrap_or_else(|| t!("Note – stored automatically").into());
        if let Some(folder) = self.tab_folders.borrow().get(id).filter(|_| features::on(Feature::Folders)) {
            tooltip.push_str(&markup_escape(&format!("\n{}", t!("Folder: {}", library::path(&self.folders.borrow(), folder)))));
        }
        entry.page.set_tooltip(&tooltip);
        if self.is_current(id) {
            self.update_window_title();
        }
        self.refresh_tree();
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

    fn new_kanban(self: &Rc<Self>) {
        let t = self.new_note();
        t.language_locked.set(true);
        t.set_text_irreversible(&kanban::template());
        t.set_language_id(Some(kanban::LANGUAGE));
        t.set_preview(false);
        self.mark_dirty(&t.id);
        self.refresh_title(&t.id);
        self.sync_ui();
    }

    fn new_mindmap(self: &Rc<Self>) {
        let t = self.new_note();
        t.language_locked.set(true);
        t.set_text_irreversible(&crate::mindmap::template());
        t.set_language_id(Some(crate::mindmap::LANGUAGE));
        t.set_preview(true);
        self.mark_dirty(&t.id);
        self.refresh_title(&t.id);
        self.sync_ui();
    }

    fn new_adventure(self: &Rc<Self>) {
        let t = self.new_note();
        t.language_locked.set(true);
        t.set_text_irreversible(&crate::adventure::template());
        t.set_language_id(Some(crate::adventure::LANGUAGE));
        t.set_preview(true);
        self.mark_dirty(&t.id);
        self.refresh_title(&t.id);
        self.sync_ui();
    }

    fn new_flashcards(self: &Rc<Self>) {
        let t = self.new_note();
        t.language_locked.set(true);
        t.set_text_irreversible(&crate::flashcards::template());
        t.set_language_id(Some(crate::flashcards::LANGUAGE));
        t.set_preview(true);
        self.mark_dirty(&t.id);
        self.refresh_title(&t.id);
        self.sync_ui();
    }

    // ===== notes library ===================================================

    /// Updates the folder tree.
    fn refresh_tree(&self) {
        if !features::on(Feature::Folders) || !self.notes_tree.root.get_visible() {
            return;
        }
        let tab_folders = self.tab_folders.borrow();
        let mut items: Vec<library::Item> = self
            .tabs
            .borrow()
            .iter()
            .filter(|(_, e)| !matches!(&e.tab, Tab::Text(t) if t.transient.get()))
            .map(|(id, e)| {
                let id = id.clone();
                let style = TabStyle::of(&e.tab);
                library::Item {
                    title: e.tab.title(),
                    folder: tab_folders.get(&id).cloned(),
                    trashed: false,
                    open: true,
                    file: e.tab.path().is_some(),
                    missing: false,
                    icon: style.icon(),
                    id,
                }
            })
            .collect();
        items.extend(self.library.borrow().iter().map(|m| library::Item {
            id: m.id.clone(),
            title: library_title(m),
            folder: m.folder.clone(),
            trashed: m.trashed,
            open: false,
            file: m.path.is_some(),
            missing: m.path.as_ref().is_some_and(|p| !p.exists()),
            icon: TabStyle::of_meta(m).icon(),
        }));
        drop(tab_folders);
        let rows = library::rows(&self.folders.borrow(), &items, &self.special_collapsed.borrow(), (t!("Ungrouped"), t!("Trash")));
        let current = self.current().map(|t| t.meta().id);
        self.notes_tree.set_rows(rows, current.as_deref());
    }

    fn on_tree_event(self: &Rc<Self>, event: notes_tree::Event) {
        match event {
            notes_tree::Event::Open(id) => self.open_note(&id),
            notes_tree::Event::Toggle(id) => {
                if id.starts_with(':') {
                    let mut set = self.special_collapsed.borrow_mut();
                    if !set.remove(&id) {
                        set.insert(id);
                    }
                } else if let Some(f) = self.folders.borrow_mut().iter_mut().find(|f| f.id == id) {
                    f.collapsed = !f.collapsed;
                }
                self.refresh_tree();
                self.schedule_persist();
            }
            notes_tree::Event::Move { folder, id, target } => {
                if folder {
                    let target = (target != UNGROUPED).then_some(target);
                    if target.as_deref() == Some(TRASH) {
                        return;
                    }
                    if !library::move_folder(&mut self.folders.borrow_mut(), &id, target.as_deref()) {
                        return;
                    }
                } else if target == TRASH {
                    self.trash_note(&id);
                    return;
                } else {
                    self.set_note_folder(&id, (target != UNGROUPED).then_some(target));
                }
                self.refresh_tree();
                self.schedule_persist();
            }
        }
    }

    /// Opens the note a wikilink (`[[Title]]`) points to: an open tab or a
    /// note of the library with that title.
    fn open_note_titled(self: &Rc<Self>, title: &str) {
        let wanted = title.trim();
        let wanted = wanted.rsplit('/').next().unwrap_or(wanted);
        let wanted = wanted.strip_suffix(".md").unwrap_or(wanted).to_lowercase();
        let open = self.tabs.borrow().iter().find(|(_, e)| e.tab.title().to_lowercase() == wanted).map(|(id, _)| id.clone());
        let stored = || {
            self.library
                .borrow()
                .iter()
                .filter(|m| !m.trashed)
                .find(|m| m.title.as_deref().or(m.label.as_deref()).is_some_and(|t| t.to_lowercase() == wanted))
                .map(|m| m.id.clone())
        };
        match open.or_else(stored) {
            Some(id) => self.open_note(&id),
            None => self.toast(&t!("No note named “{}”", title.trim())),
        }
    }

    /// Imports an Obsidian vault into the notes library (see `obsidian.rs`).
    fn import_obsidian(self: &Rc<Self>) {
        if !features::on(Feature::Folders) {
            return;
        }
        let dialog = gtk::FileDialog::builder().title(t!("Import Obsidian Vault")).modal(true).build();
        let this = self.clone();
        glib::spawn_future_local(async move {
            let Ok(dir) = dialog.select_folder_future(Some(&this.win)).await else { return };
            let Some(root) = dir.path() else { return };
            let root = root.canonicalize().unwrap_or(root);
            let name = root.file_name().map_or_else(|| "Obsidian".to_owned(), |n| n.to_string_lossy().into_owned());
            let dest = crate::obsidian::attachment_dir(&name);
            this.toast(&t!("Importing “{}”…", name));
            let source = root.clone();
            let result = gio::spawn_blocking(move || {
                let scan = crate::obsidian::scan(&source, &dest)?;
                let copied = crate::obsidian::copy_attachments(&scan, &dest)?;
                Ok::<_, String>((scan, copied))
            })
            .await
            .unwrap_or_else(|_| Err("import failed".into()));
            match result {
                Ok((scan, copied)) => this.add_vault(&root, scan, copied),
                Err(e) => this.toast(&t!("Import failed: {}", e)),
            }
        });
    }

    /// Puts the notes of a scanned vault into its library folder; notes of
    /// an earlier import of the same vault are skipped.
    fn add_vault(self: &Rc<Self>, root: &Path, scan: crate::obsidian::Scan, attachments: usize) {
        if scan.notes.is_empty() {
            self.toast(&t!("No Markdown notes found in “{}”", scan.name));
            return;
        }
        let mut record = self
            .settings
            .borrow()
            .obsidian_vaults
            .iter()
            .find(|v| v.path == root)
            .cloned()
            .unwrap_or_else(|| store::ObsidianVault { path: root.to_path_buf(), ..Default::default() });
        let (mut added, mut skipped) = (0, 0);
        {
            let mut folders = self.folders.borrow_mut();
            if !folders.iter().any(|f| f.id == record.folder) {
                let name = library::unique_name(&folders, None, &scan.name, None);
                record.folder = store::new_id();
                folders.push(Folder { id: record.folder.clone(), name, parent: None, collapsed: false });
            }
            for note in scan.notes {
                if record.notes.contains_key(&note.rel) {
                    skipped += 1;
                    continue;
                }
                let folder = library::ensure_path(&mut folders, Some(&record.folder), &note.dirs, store::new_id);
                let id = store::new_id();
                if let Err(e) = store::write_note(&id, &note.content) {
                    self.toast(&t!("Cannot store note: {}", e));
                    break;
                }
                self.library.borrow_mut().push(TabMeta {
                    id: id.clone(),
                    kind: TabKind::Text,
                    title: Some(note.title.clone()),
                    language: Some("markdown".into()),
                    language_locked: true,
                    wrap: true,
                    preview: true,
                    folder,
                    label: Some(note.title),
                    ..Default::default()
                });
                record.notes.insert(note.rel, id);
                added += 1;
            }
        }
        {
            let mut settings = self.settings.borrow_mut();
            settings.obsidian_vaults.retain(|v| v.path != record.path);
            settings.obsidian_vaults.push(record);
        }
        self.refresh_tree();
        self.schedule_persist();
        let mut message = t!("Imported {} notes from “{}”", added, scan.name);
        match attachments {
            0 => {}
            1 => message.push_str(t!(", 1 attachment")),
            n => message.push_str(&t!(", {} attachments", n)),
        }
        if skipped > 0 {
            message.push_str(&t!(" ({} were already imported)", skipped));
        }
        self.toast(&message);
    }

    /// Opens a note of the library (or switches to its tab).
    fn open_note(self: &Rc<Self>, id: &str) {
        let page = self.tabs.borrow().get(id).map(|e| e.page.clone());
        if let Some(page) = page {
            self.view.set_selected_page(&page);
            return;
        }
        let Some(pos) = self.library.borrow().iter().position(|m| m.id == id) else { return };
        let mut meta = self.library.borrow()[pos].clone();
        if let Some(p) = meta.path.as_ref().filter(|p| !p.exists()) {
            self.toast(&t!("File not found: {}", p.display()));
            return;
        }
        meta.trashed = false;
        let content = (meta.path.is_none() && meta.lock.is_none() && meta.kind == TabKind::Text)
            .then(|| store::read_note(id))
            .flatten();
        let removed = self.library.borrow_mut().remove(pos);
        match self.restore_tab(meta, content) {
            Some(page) => self.view.set_selected_page(&page),
            None => self.library.borrow_mut().insert(pos, removed),
        }
        self.refresh_tree();
        self.schedule_persist();
    }

    /// Closing a tab with the notes library: notes stay in it (in their
    /// folder), files only when they are in a folder.
    fn forget_into_library(self: &Rc<Self>, tab: &Tab) {
        let mut meta = tab.meta();
        let id = meta.id.clone();
        self.tabs.borrow_mut().remove(&id);
        self.dirty.borrow_mut().remove(&id);
        meta.folder = self.tab_folders.borrow_mut().remove(&id);
        meta.trashed = self.trash_on_close.borrow_mut().remove(&id);
        meta.modified = false;
        let keep = match tab {
            Tab::Text(t) if t.path.borrow().is_none() => {
                if let Some(sealed) = t.sealed_content() {
                    let _ = store::write_locked(&id, &sealed);
                    meta.label = Some(t.custom_title.borrow().clone().unwrap_or_else(|| t!("Protected note").to_owned()));
                    true
                } else {
                    let text = t.text();
                    if text.trim().is_empty() && t.custom_title.borrow().is_none() {
                        store::delete_note(&id);
                        false
                    } else {
                        if let Err(e) = store::write_note(&id, &text) {
                            self.toast(&t!("Cannot store note: {}", e));
                        }
                        meta.label = Some(t.title());
                        true
                    }
                }
            }
            _ => {
                meta.label = Some(tab.title());
                meta.folder.is_some()
            }
        };
        if keep {
            self.library.borrow_mut().push(meta.clone());
        }
        if keep || meta.path.is_some() {
            self.closed.borrow_mut().push((meta, None));
        }
        self.refresh_tree();
        self.schedule_persist();
    }

    fn set_note_folder(&self, id: &str, folder: Option<String>) {
        if self.tabs.borrow().contains_key(id) {
            match folder {
                Some(f) => {
                    self.tab_folders.borrow_mut().insert(id.to_owned(), f);
                }
                None => {
                    self.tab_folders.borrow_mut().remove(id);
                }
            }
        } else if let Some(m) = self.library.borrow_mut().iter_mut().find(|m| m.id == id) {
            m.folder = folder;
            m.trashed = false;
        }
    }

    /// Moves a note to the trash (closing its tab).
    fn trash_note(self: &Rc<Self>, id: &str) {
        let page = self.tabs.borrow().get(id).map(|e| e.page.clone());
        if let Some(page) = page {
            if self.tabs.borrow().get(id).and_then(|e| e.tab.path()).is_some() {
                return;
            }
            self.trash_on_close.borrow_mut().insert(id.to_owned());
            self.view.close_page(&page);
        } else if let Some(m) = self.library.borrow_mut().iter_mut().find(|m| m.id == id && m.path.is_none()) {
            m.trashed = true;
        }
        self.refresh_tree();
        self.schedule_persist();
    }

    fn restore_note(self: &Rc<Self>, id: &str) {
        if let Some(m) = self.library.borrow_mut().iter_mut().find(|m| m.id == id) {
            m.trashed = false;
        }
        self.refresh_tree();
        self.schedule_persist();
    }

    /// Deletes notes of the trash for good (after asking).
    fn delete_notes(self: &Rc<Self>, ids: Vec<String>) {
        if ids.is_empty() {
            return;
        }
        let heading = if ids.len() == 1 {
            let title = self.library.borrow().iter().find(|m| m.id == ids[0]).map(library_title).unwrap_or_default();
            t!("Delete “{}” permanently?", title)
        } else {
            t!("Delete {} notes permanently?", ids.len())
        };
        let dialog = adw::AlertDialog::new(Some(&heading), Some(t!("This cannot be undone.")));
        dialog.add_responses(&[("cancel", t!("_Cancel")), ("delete", t!("_Delete"))]);
        dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
        dialog.set_close_response("cancel");
        let this = self.clone();
        glib::spawn_future_local(async move {
            if dialog.choose_future(&this.win).await != "delete" {
                return;
            }
            this.library.borrow_mut().retain(|m| {
                let delete = ids.contains(&m.id);
                if delete && m.path.is_none() {
                    store::delete_note(&m.id);
                }
                !delete
            });
            this.closed.borrow_mut().retain(|(m, _)| !ids.contains(&m.id));
            this.refresh_tree();
            this.schedule_persist();
        });
    }

    /// Removes a file from the library (the file itself stays).
    fn remove_link(self: &Rc<Self>, id: &str) {
        if self.tabs.borrow().contains_key(id) {
            self.tab_folders.borrow_mut().remove(id);
        } else {
            self.library.borrow_mut().retain(|m| m.id != id);
        }
        self.refresh_tree();
        self.schedule_persist();
    }

    /// Asks for a folder name.
    async fn ask_folder_name(&self, heading: &str, initial: &str) -> Option<String> {
        let dialog = adw::AlertDialog::new(Some(heading), None);
        let entry = gtk::Entry::builder().text(initial).activates_default(true).build();
        dialog.set_extra_child(Some(&entry));
        dialog.add_responses(&[("cancel", t!("_Cancel")), ("ok", t!("_OK"))]);
        dialog.set_response_appearance("ok", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("ok"));
        dialog.set_close_response("cancel");
        entry.grab_focus();
        let answer = dialog.choose_future(&self.win).await;
        let name = entry.text().trim().to_owned();
        (answer == "ok" && !name.is_empty()).then_some(name)
    }

    fn new_folder(self: &Rc<Self>, parent: String) {
        let parent = (!parent.is_empty() && !parent.starts_with(':')).then_some(parent);
        let this = self.clone();
        glib::spawn_future_local(async move {
            let Some(name) = this.ask_folder_name(t!("New Folder"), "").await else { return };
            {
                let mut folders = this.folders.borrow_mut();
                let name = library::unique_name(&folders, parent.as_deref(), &name, None);
                if let Some(p) = folders.iter_mut().find(|f| Some(&f.id) == parent.as_ref()) {
                    p.collapsed = false;
                }
                folders.push(Folder { id: store::new_id(), name, parent, collapsed: false });
            }
            this.refresh_tree();
            this.schedule_persist();
        });
    }

    fn rename_folder(self: &Rc<Self>, id: String) {
        let Some(current) = self.folders.borrow().iter().find(|f| f.id == id).map(|f| f.name.clone()) else { return };
        let this = self.clone();
        glib::spawn_future_local(async move {
            let Some(name) = this.ask_folder_name(t!("Rename Folder"), &current).await else { return };
            {
                let mut folders = this.folders.borrow_mut();
                let parent = folders.iter().find(|f| f.id == id).and_then(|f| f.parent.clone());
                let name = library::unique_name(&folders, parent.as_deref(), &name, Some(&id));
                if let Some(f) = folders.iter_mut().find(|f| f.id == id) {
                    f.name = name;
                }
            }
            this.refresh_tree();
            this.schedule_persist();
        });
    }

    /// Removes a folder; its notes and sub-folders move one level up.
    fn delete_folder(self: &Rc<Self>, id: &str) {
        let Some(parent) = library::remove_folder(&mut self.folders.borrow_mut(), id) else { return };
        for f in self.tab_folders.borrow_mut().values_mut().filter(|f| *f == id) {
            match &parent {
                Some(p) => *f = p.clone(),
                None => f.clear(),
            }
        }
        self.tab_folders.borrow_mut().retain(|_, f| !f.is_empty());
        for m in self.library.borrow_mut().iter_mut().filter(|m| m.folder.as_deref() == Some(id)) {
            m.folder = parent.clone();
        }
        self.refresh_tree();
        self.schedule_persist();
    }

    /// The help section for the current tab and cursor (F1).
    fn help_topic(&self) -> Option<&'static str> {
        use crate::help::{Cursor, Doc};
        let in_tree = gtk::prelude::GtkWindowExt::focus(&self.win).is_some_and(|f| f.is_ancestor(&self.notes_tree.root));
        if in_tree && features::on(Feature::Folders) {
            return Some("folders");
        }
        let t = match self.current()? {
            Tab::Text(t) => t,
            _ => return Some(crate::help::topic(Doc::Image, Cursor::default(), false)),
        };
        if self.tab_folders.borrow().get(&t.id).is_some_and(|f| f == TEMPLATES_FOLDER) {
            return Some("note-templates");
        }
        let language = t.language_id();
        let doc = match t.preview_kind() {
            Some(PreviewKind::Flashcards) => Doc::Flashcards,
            Some(PreviewKind::Slides) => Doc::Slides,
            Some(PreviewKind::Kanban) => Doc::Kanban,
            Some(PreviewKind::Mindmap) => Doc::Mindmap,
            Some(PreviewKind::Adventure) => Doc::Adventure,
            Some(PreviewKind::Markdown) => Doc::Markdown,
            Some(PreviewKind::Rst | PreviewKind::Html | PreviewKind::PlantUml) => Doc::Preview,
            None if t.is_calc() => Doc::Calc,
            None => match language.as_deref() {
                None => Doc::Note,
                Some(l) => Doc::Code(l),
            },
        };
        let fence = t.fence_at_cursor();
        let cursor = Cursor { fence: fence.as_deref(), in_table: t.table_at_cursor().is_some(), on_task: t.on_task_line() };
        Some(crate::help::topic(doc, cursor, t.emmet_syntax().is_some()))
    }

    /// Text tabs with flash cards.
    fn flashcard_tabs(&self) -> Vec<Rc<EditorTab>> {
        self.text_tabs().into_iter().filter(|t| t.preview_kind() == Some(PreviewKind::Flashcards)).collect()
    }

    /// Learns the deck `deck` (path; empty: all) of the flash cards in `tabs`:
    /// due and new cards, or all of them in `practice` (schedule unchanged).
    fn study(self: &Rc<Self>, tabs: Vec<Rc<EditorTab>>, deck: &str, practice: bool) {
        use crate::flashcards::{self, Card};
        use crate::learning::{self, CardRef};
        if tabs.is_empty() {
            return;
        }
        let today = learning::today();
        let docs: Vec<(String, flashcards::Doc, bool)> =
            tabs.iter().map(|t| (t.card_doc_id(), t.flashcards(), t.lock().is_some())).collect();
        // Edited cards keep their progress.
        {
            let mut store = self.shared.learning.borrow_mut();
            let mut changed = false;
            for (id, doc, locked) in &docs {
                let texts: Vec<String> = doc.cards.iter().map(Card::front_text).collect();
                let refs: Vec<CardRef<'_>> = doc.cards.iter().zip(&texts).map(|(c, t)| CardRef { key: &c.key, front: t }).collect();
                changed |= store.adopt(id, &refs, !locked);
            }
            if changed {
                let _ = store.save();
            }
        }
        let store = self.shared.learning.borrow();
        let pairs: Vec<(&Card, Option<&learning::CardState>)> =
            docs.iter().flat_map(|(_, doc, _)| doc.cards.iter().map(|c| (c, store.state(&c.key)))).collect();
        let keys: std::collections::HashSet<&str> = pairs.iter().map(|(c, _)| c.key.as_str()).collect();
        let settings = docs[0].1.settings.clone();
        let chosen = flashcards::session_cards(&pairs, deck, practice, store.new_today(&keys, today), settings.new_per_day, today);
        if chosen.is_empty() {
            drop(store);
            self.toast(if practice { t!("This deck has no cards") } else { t!("Nothing to learn right now – practise instead?") });
            return;
        }
        // Which document each card comes from (for the progress).
        let mut owners: HashMap<String, (String, bool)> = HashMap::new();
        for (id, doc, locked) in &docs {
            for c in &doc.cards {
                owners.entry(c.key.clone()).or_insert((id.clone(), *locked));
            }
        }
        let cards: Vec<flashcards::StudyCard> =
            chosen.iter().map(|c| flashcards::study_card(c, store.state(&c.key), practice, today)).collect();
        drop(store);
        let title = if deck.is_empty() {
            docs[0].1.cards.first().and_then(|c| c.deck.first()).cloned().unwrap_or_else(|| tabs[0].title())
        } else {
            deck.replace('/', " › ")
        };
        let colors = flashcards::colors(&settings, self.shared.theme.get().dark);
        let page = flashcards::study_page(&title, &cards, &colors, practice);
        if let Some(running) = self.study.borrow_mut().take() {
            running.close();
        }
        let shared = self.shared.clone();
        let on_answer = move |a: crate::study::Answer| {
            let (doc, locked) = owners.get(&a.key).cloned().unwrap_or_default();
            let text = if locked { "" } else { a.text.as_str() };
            let mut store = shared.learning.borrow_mut();
            let answered =
                learning::Answered { key: &a.key, doc: &doc, front: text, rating: a.rating, ms: a.ms, practice: a.practice };
            store.review(&answered, learning::today(), learning::now_unix());
            if let Err(e) = store.save() {
                eprintln!("quicknotes: cannot save the learning progress: {e}");
            }
        };
        let weak = self.weak();
        let on_close = move || {
            let Some(this) = weak.upgrade() else { return };
            this.study.borrow_mut().take();
            for t in this.flashcard_tabs() {
                t.refresh_preview();
            }
            this.refresh_cards_overview();
        };
        let study = crate::study::Study::open(&self.win, &title, &page, tabs[0].base_uri(), on_answer, on_close);
        *self.study.borrow_mut() = Some(study);
    }

    /// The decks of all open flash card tabs, in a window of their own.
    fn show_cards_overview(self: &Rc<Self>) {
        if let Some((win, _)) = self.cards_overview.borrow().as_ref() {
            win.present();
        }
        if self.cards_overview.borrow().is_some() {
            self.refresh_cards_overview();
            return;
        }
        let webview = webkit6::WebView::new();
        webview.set_vexpand(true);
        webview.set_background_color(&crate::editor::preview_bg(self.shared.theme.get()));
        if let Some(ucm) = webview.user_content_manager() {
            ucm.register_script_message_handler("qn", None);
            let weak = self.weak();
            ucm.connect_script_message_received(Some("qn"), move |_, value| {
                let Some(this) = weak.upgrade() else { return };
                let Ok(msg) = serde_json::from_str::<serde_json::Value>(&value.to_str()) else { return };
                if msg.get("type").and_then(|v| v.as_str()) == Some("learn") {
                    let deck = msg.get("deck").and_then(|v| v.as_str()).unwrap_or("").to_owned();
                    let practice = msg.get("mode").and_then(|v| v.as_str()) == Some("practice");
                    let tabs = this.flashcard_tabs();
                    this.study(tabs, &deck, practice);
                }
            });
        }
        let header = adw::HeaderBar::new();
        let content = adw::ToolbarView::new();
        content.add_top_bar(&header);
        content.set_content(Some(&webview));
        let win = adw::Window::builder()
            .title(t!("Learning Overview"))
            .default_width(820)
            .default_height(720)
            .transient_for(&self.win)
            .content(&content)
            .build();
        let weak = self.weak();
        win.connect_close_request(move |_| {
            if let Some(this) = weak.upgrade() {
                this.cards_overview.borrow_mut().take();
            }
            glib::Propagation::Proceed
        });
        *self.cards_overview.borrow_mut() = Some((win.clone().upcast(), webview));
        self.refresh_cards_overview();
        win.present();
    }

    fn refresh_cards_overview(&self) {
        let Some((_, webview)) = self.cards_overview.borrow().clone() else { return };
        let docs: Vec<(String, crate::flashcards::Doc)> =
            self.flashcard_tabs().iter().map(|t| (t.card_doc_id(), t.flashcards())).collect();
        let store = self.shared.learning.borrow();
        let studied: Vec<crate::flashcards::Studied<'_>> =
            docs.iter().map(|(id, doc)| crate::flashcards::Studied::new(doc, id, &store)).collect();
        let mut body = crate::flashcards::overview_html(&studied, &store, crate::learning::today(), false);
        if docs.is_empty() {
            body = format!("<p>{}</p>", html::escape(t!("No flash cards are open. Create some with Main menu → New Flash Cards.")));
        }
        drop(store);
        let page = html::app_page(t!("Learning Overview"), &body, self.shared.theme.get());
        webview.load_html(&page, None);
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

    // ===== sync (GitHub, Nextcloud, WebDAV, folder, S3) ======================

    /// The connected sync target, if the features it needs are on.
    fn active_sync(&self) -> Option<ActiveSync> {
        if !features::on(Feature::Sync) || !features::on(Feature::Folders) {
            return None;
        }
        let s = self.settings.borrow();
        if !s.github_account.is_empty() && !s.github_repo.is_empty() {
            return Some(ActiveSync::GitHub(s.github_repo.clone()));
        }
        s.sync_target.clone().map(ActiveSync::Remote)
    }

    fn github_connected(&self) -> bool {
        self.active_sync().is_some()
    }

    /// Syncs every 5 minutes while automatic sync is on.
    fn sync_timer(self: &Rc<Self>) {
        let weak = self.weak();
        glib::timeout_add_seconds_local(300, move || {
            let Some(this) = weak.upgrade() else { return glib::ControlFlow::Break };
            if this.github_ready() && !this.sync_busy.get() && !this.sync_closing.get() && !this.locking.get() {
                this.github_sync(|this, summary| {
                    if let Some(s) = summary.filter(|s| !s.is_empty()) {
                        this.toast(&s);
                    }
                });
            }
            glib::ControlFlow::Continue
        });
    }

    /// Automatic sync (start, close) is on.
    fn github_ready(&self) -> bool {
        self.github_connected() && self.settings.borrow().github_sync
    }

    /// An open note as it is synced; `None` for files and tabs that are
    /// still loading or unlocked (their content is in flux).
    fn sync_note(&self, t: &EditorTab) -> Option<github::Note> {
        if t.path.borrow().is_some() || t.loading.get() {
            return None;
        }
        let (lock, content, label) = match t.lock_state() {
            LockState::Unlocked => return None,
            LockState::Locked => {
                (t.lock(), b64().encode(t.sealed_content()?), t.custom_title.borrow().clone())
            }
            LockState::Open => (None, t.text(), Some(t.title())),
        };
        let meta = github::Meta {
            title: t.custom_title.borrow().clone(),
            language: t.language_id(),
            language_locked: t.language_locked.get(),
            folder: self.tab_folders.borrow().get(&t.id).cloned(),
            trashed: false,
        };
        Some(github::Note { id: t.id.clone(), meta, label, lock, content })
    }

    /// All notes (open or in the library) and the ones to leave alone.
    fn sync_notes(&self, state: &github::State) -> (Vec<github::Note>, HashSet<String>) {
        let mut notes = Vec::new();
        let mut skip = HashSet::new();
        for t in self.text_tabs() {
            if t.path.borrow().is_some() || t.transient.get() {
                continue;
            }
            // A fresh empty note is not worth sending (it disappears when closed).
            let fresh = t.lock().is_none() && t.is_empty() && t.custom_title.borrow().is_none();
            match self.sync_note(&t) {
                Some(n) if !(fresh && !state.notes.contains_key(&n.id)) => notes.push(n),
                _ => {
                    skip.insert(t.id.clone());
                }
            }
        }
        for m in self.library.borrow().iter().filter(|m| m.path.is_none() && m.kind == TabKind::Text) {
            let content = match m.lock {
                Some(_) => store::read_locked(&m.id).map(|sealed| b64().encode(sealed)),
                None => store::read_note(&m.id),
            };
            let Some(content) = content else {
                skip.insert(m.id.clone());
                continue;
            };
            let meta = github::Meta {
                title: m.title.clone(),
                language: m.language.clone(),
                language_locked: m.language_locked,
                folder: m.folder.clone(),
                trashed: m.trashed,
            };
            notes.push(github::Note { id: m.id.clone(), meta, label: m.label.clone(), lock: m.lock.clone(), content });
        }
        (notes, skip)
    }

    /// Fetches and sends changes after the start, once all tabs are loaded.
    fn github_startup(self: &Rc<Self>) {
        if !self.github_ready() {
            return;
        }
        let weak = self.weak();
        let tries = Cell::new(0);
        glib::timeout_add_local(Duration::from_millis(300), move || {
            let Some(this) = weak.upgrade() else { return glib::ControlFlow::Break };
            tries.set(tries.get() + 1);
            if this.text_tabs().iter().any(|t| t.loading.get()) && tries.get() < 60 {
                return glib::ControlFlow::Continue;
            }
            this.github_sync(|this, summary| {
                if let Some(s) = summary.filter(|s| !s.is_empty()) {
                    this.toast(&s);
                }
            });
            glib::ControlFlow::Break
        });
    }

    /// *Sync Now* from the menu or the preferences.
    fn github_sync_now(self: &Rc<Self>) {
        if !features::on(Feature::Sync) {
            return;
        }
        if !features::on(Feature::Folders) {
            self.toast(t!("Syncing needs the note folders – switch them on in the preferences (Features)"));
            return;
        }
        let Some(active) = self.active_sync() else {
            self.toast(t!("Set up syncing in the preferences (Sync)"));
            return;
        };
        // Errors of a sync by hand are always shown.
        self.sync_failed.set(false);
        let name = active.name();
        self.github_sync(move |this, summary| match summary {
            Some(s) if !s.is_empty() => this.toast(&s),
            Some(_) => this.toast(&t!("Synced with {} – everything is up to date", name)),
            None => {}
        });
    }

    /// Runs a sync in a worker thread; `done` gets a summary of what changed
    /// (empty: nothing), or `None` when it failed (an error was shown).
    fn github_sync(self: &Rc<Self>, done: impl FnOnce(&Rc<Self>, Option<String>) + 'static) {
        if self.sync_busy.get() {
            // Wait for the running sync (e.g. the one at start when closing early).
            let weak = self.weak();
            glib::timeout_add_local_once(Duration::from_millis(250), move || {
                if let Some(this) = weak.upgrade() {
                    this.github_sync(done);
                }
            });
            return;
        }
        let Some(active) = self.active_sync() else {
            done(self, None);
            return;
        };
        self.sync_busy.set(true);
        self.persist();
        let state = github::load_state();
        let (notes, skip) = self.sync_notes(&state);
        let snapshot: HashMap<String, String> = notes.iter().map(|n| (n.id.clone(), n.hash())).collect();
        let job = github::Job {
            repo: active.id(),
            notes,
            skip,
            folders: self.folders.borrow().iter().map(github::SyncFolder::from).collect(),
            state,
            stamp: glib::DateTime::now_local()
                .and_then(|d| d.format("%Y-%m-%d %H:%M"))
                .map(|s| s.to_string())
                .unwrap_or_default(),
        };
        let this = self.clone();
        let name = active.name();
        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(move || match &active {
                ActiveSync::GitHub(_) => github::sync(job),
                ActiveSync::Remote(target) => crate::remote::sync(target, job),
            })
            .await;
            this.sync_busy.set(false);
            // Automatic syncs show an error once, not every five minutes.
            let report = !this.sync_failed.replace(!matches!(result, Ok(Ok(_))));
            let summary = match result {
                Ok(Ok(outcome)) => Some(this.github_apply(outcome, &snapshot, name)),
                Ok(Err(github::Error::Auth(e))) => {
                    eprintln!("quicknotes: {name}: {e}");
                    if report {
                        this.toast(&t!("{}: please sign in again in the preferences (Sync)", name));
                    }
                    None
                }
                Ok(Err(e)) => {
                    if report {
                        this.toast(&t!("Sync with {} failed: {}", name, e));
                    }
                    None
                }
                Err(_) => None,
            };
            done(&this, summary);
        });
    }

    /// The open text tab of a note.
    fn text_tab(&self, id: &str) -> Option<Rc<EditorTab>> {
        match self.tabs.borrow().get(id).map(|e| e.tab.clone()) {
            Some(Tab::Text(t)) => Some(t),
            _ => None,
        }
    }

    /// Applies what a sync brought; a summary of it for a toast.
    fn github_apply(self: &Rc<Self>, mut outcome: github::Outcome, snapshot: &HashMap<String, String>, name: &str) -> String {
        // Tabs edited while the sync ran keep the edit.
        let edited = |t: &EditorTab| self.sync_note(t).map(|n| n.hash()).as_ref() != snapshot.get(&t.id);
        {
            let mut folders = self.folders.borrow_mut();
            let collapsed: HashSet<String> = folders.iter().filter(|f| f.collapsed).map(|f| f.id.clone()).collect();
            *folders = outcome
                .folders
                .iter()
                .map(|f| Folder { id: f.id.clone(), name: f.name.clone(), parent: f.parent.clone(), collapsed: collapsed.contains(&f.id) })
                .collect();
        }
        let (mut received, mut deleted, mut conflicts) = (0, 0, outcome.conflicts);
        for change in std::mem::take(&mut outcome.changes) {
            match change {
                github::Change::Put { id, meta, label, lock, content } => {
                    received += 1;
                    if let Some(t) = self.text_tab(&id) {
                        if content.is_some() && edited(&t) {
                            // The repository's version goes next to it (and
                            // is sent with the next sync).
                            let title = Some(format!("{} ({})", meta.title.clone().or(label.clone()).unwrap_or_else(|| t.title()), t!("{} version", name)));
                            let meta = github::Meta { title, ..meta };
                            self.add_synced_note(store::new_id(), meta, label, lock, content);
                            conflicts += 1;
                        } else {
                            self.apply_to_tab(&t, meta, lock, content);
                        }
                        continue;
                    }
                    let known = self.library.borrow().iter().any(|m| m.id == id);
                    if known {
                        self.apply_to_library(&id, meta, label, lock, content);
                    } else if content.is_some() {
                        self.add_synced_note(id, meta, label, lock, content);
                    }
                }
                github::Change::Delete { id } => {
                    let page = self.tabs.borrow().get(&id).map(|e| e.page.clone());
                    match (page, self.text_tab(&id)) {
                        (Some(_), Some(t)) if edited(&t) => {
                            // Kept; sent again as a new note next time.
                            outcome.state.notes.remove(&id);
                            continue;
                        }
                        (Some(page), _) => {
                            self.sync_discard.borrow_mut().insert(id.clone());
                            self.view.close_page(&page);
                        }
                        (None, _) => {
                            self.library.borrow_mut().retain(|m| m.id != id);
                            store::delete_note(&id);
                        }
                    }
                    self.closed.borrow_mut().retain(|(m, _)| m.id != id);
                    deleted += 1;
                }
            }
        }
        github::save_state(&outcome.state);
        self.refresh_tree();
        self.persist_all();
        self.sync_ui();
        let mut parts = Vec::new();
        if received > 0 {
            parts.push(t!("{} received", received));
        }
        if outcome.sent > 0 {
            parts.push(t!("{} sent", outcome.sent));
        }
        if deleted > 0 {
            parts.push(t!("{} deleted elsewhere", deleted));
        }
        if conflicts > 0 {
            parts.push(t!("{} conflicts kept as copies", conflicts));
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!("{name}: {}", parts.join(", "))
        }
    }

    /// A synced change to an open note.
    fn apply_to_tab(self: &Rc<Self>, t: &Rc<EditorTab>, meta: github::Meta, lock: Option<crypto::Lock>, content: Option<String>) {
        *t.custom_title.borrow_mut() = meta.title.clone();
        t.language_locked.set(meta.language_locked);
        if t.language_id() != meta.language {
            t.set_language_id(meta.language.as_deref());
        }
        match &meta.folder {
            Some(f) => {
                self.tab_folders.borrow_mut().insert(t.id.clone(), f.clone());
            }
            None => {
                self.tab_folders.borrow_mut().remove(&t.id);
            }
        }
        if let Some(content) = content {
            match lock {
                Some(lock) => {
                    if let Ok(sealed) = b64().decode(content.as_bytes()) {
                        t.set_locked(lock, sealed);
                        self.update_lock_icon(t);
                    }
                }
                None => {
                    t.drop_lock();
                    let (mut start, mut end) = t.buffer.bounds();
                    t.buffer.begin_user_action();
                    t.buffer.delete(&mut start, &mut end);
                    t.buffer.insert(&mut start, &content);
                    t.buffer.end_user_action();
                }
            }
        }
        self.mark_dirty(&t.id);
        self.refresh_title(&t.id);
        if meta.trashed {
            self.trash_note(&t.id);
        }
    }

    /// A synced change to a note of the library (not open).
    fn apply_to_library(&self, id: &str, meta: github::Meta, label: Option<String>, lock: Option<crypto::Lock>, content: Option<String>) {
        if let Some(content) = &content {
            Self::store_synced(id, lock.is_some(), content);
        }
        if let Some(m) = self.library.borrow_mut().iter_mut().find(|m| m.id == id) {
            m.title = meta.title;
            m.language = meta.language;
            m.language_locked = meta.language_locked;
            m.folder = meta.folder;
            m.trashed = meta.trashed;
            if content.is_some() {
                m.lock = lock;
                m.label = label;
            }
        }
    }

    /// Stores a received note's content.
    fn store_synced(id: &str, locked: bool, content: &str) {
        let result = if locked {
            match b64().decode(content.as_bytes()) {
                Ok(sealed) => store::write_locked(id, &sealed),
                Err(e) => Err(std::io::Error::other(e)),
            }
        } else {
            store::write_note(id, content)
        };
        if let Err(e) = result {
            eprintln!("quicknotes: cannot store synced note {id}: {e}");
        }
    }

    /// A note that came from the repository: into the library.
    fn add_synced_note(&self, id: String, meta: github::Meta, label: Option<String>, lock: Option<crypto::Lock>, content: Option<String>) {
        let Some(content) = content else { return };
        Self::store_synced(&id, lock.is_some(), &content);
        let label = label.or_else(|| lock.is_some().then(|| t!("Protected note").to_owned()));
        self.library.borrow_mut().push(TabMeta {
            id,
            kind: TabKind::Text,
            title: meta.title,
            language: meta.language,
            language_locked: meta.language_locked,
            wrap: true,
            lock,
            folder: meta.folder,
            trashed: meta.trashed,
            label,
            ..Default::default()
        });
    }

    /// Sends the notes to GitHub before the window closes.
    fn close_with_sync(self: &Rc<Self>) {
        let dialog = adw::AlertDialog::new(
            Some(t!("Syncing with GitHub…")),
            Some(t!("Your notes are sent to GitHub. The window closes afterwards.")),
        );
        let spinner = gtk::Spinner::builder().spinning(true).width_request(32).height_request(32).build();
        dialog.set_extra_child(Some(&spinner));
        dialog.add_responses(&[("skip", t!("Close Without Syncing"))]);
        dialog.set_close_response("skip");
        let weak = self.weak();
        dialog.connect_response(None, move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.win.close();
            }
        });
        dialog.present(Some(&self.win));
        let finished = Rc::new(Cell::new(false));
        let finish = {
            let dialog = dialog.clone();
            move |this: &Rc<Self>| {
                if !finished.replace(true) {
                    dialog.force_close();
                    this.win.close();
                }
            }
        };
        let on_timeout = finish.clone();
        let weak = self.weak();
        glib::timeout_add_local_once(Duration::from_secs(60), move || {
            if let Some(this) = weak.upgrade() {
                on_timeout(&this);
            }
        });
        self.github_sync(move |this, _| finish(this));
    }

    /// Signs in (device code or token) and stores the token.
    fn github_sign_in(self: &Rc<Self>, use_token: bool, done: Box<dyn Fn(Result<String, String>)>) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let token = if use_token { this.ask_token().await } else { this.device_sign_in().await };
            let token = match token {
                Ok(t) => t,
                Err(e) => {
                    done(Err(e));
                    return;
                }
            };
            let check = token.clone();
            match gio::spawn_blocking(move || github::user(&check)).await {
                Ok(Ok(login)) => {
                    if let Err(e) = github::secret::store(github::secret::TOKEN, &token) {
                        done(Err(t!("Cannot store the token in the keyring: {}", e)));
                        return;
                    }
                    this.settings.borrow_mut().github_account = login.clone();
                    this.persist_all();
                    this.sync_ui();
                    done(Ok(login));
                }
                Ok(Err(e)) => done(Err(e.to_string())),
                Err(_) => done(Err(t!("Sign-in failed").into())),
            }
        });
    }

    /// Asks for a personal access token.
    async fn ask_token(&self) -> Result<Zeroizing<String>, String> {
        let dialog = adw::AlertDialog::new(
            Some(t!("Personal Access Token")),
            Some(t!("Create a fine-grained token for your notes repository with the permission “Contents: Read and write” and paste it here. It is kept in the keyring.")),
        );
        let fields = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let entry = gtk::PasswordEntry::builder()
            .show_peek_icon(true)
            .placeholder_text(t!("Token"))
            .activates_default(true)
            .build();
        fields.append(&entry);
        fields.append(&gtk::LinkButton::with_label(github::TOKEN_PAGE, t!("Create a token on GitHub")));
        dialog.set_extra_child(Some(&fields));
        dialog.add_responses(&[("cancel", t!("_Cancel")), ("ok", t!("_Sign In"))]);
        dialog.set_response_appearance("ok", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("ok"));
        dialog.set_close_response("cancel");
        dialog.set_response_enabled("ok", false);
        let d = dialog.clone();
        entry.connect_changed(move |e| d.set_response_enabled("ok", !e.text().trim().is_empty()));
        dialog.set_focus(Some(&entry));
        let response = dialog.choose_future(&self.win).await;
        let token = Zeroizing::new(entry.text().trim().to_owned());
        entry.set_text("");
        if response == "ok" {
            Ok(token)
        } else {
            Err(t!("Sign-in cancelled").into())
        }
    }

    /// The device flow: shows the code, waits until it was entered on github.com.
    async fn device_sign_in(&self) -> Result<Zeroizing<String>, String> {
        let code = match gio::spawn_blocking(github::device_start).await {
            Ok(Ok(c)) => c,
            Ok(Err(e)) => return Err(e.to_string()),
            Err(_) => return Err(t!("Sign-in failed").into()),
        };
        let dialog = adw::AlertDialog::new(
            Some(t!("Sign In to GitHub")),
            Some(t!("Enter this code on GitHub to give QuickNotes access to your repositories. The dialog closes by itself afterwards.")),
        );
        let fields = gtk::Box::new(gtk::Orientation::Vertical, 12);
        let label = gtk::Label::builder().label(&code.user_code).selectable(true).build();
        label.add_css_class("title-1");
        fields.append(&label);
        let open = gtk::Button::builder().label(t!("Copy Code and Open GitHub")).halign(gtk::Align::Center).build();
        open.add_css_class("suggested-action");
        open.add_css_class("pill");
        let (user_code, uri, win) = (code.user_code.clone(), code.verification_uri.clone(), self.win.clone());
        open.connect_clicked(move |_| {
            win.clipboard().set_text(&user_code);
            gtk::UriLauncher::new(&uri).launch(Some(&win), None::<&gio::Cancellable>, |_| {});
        });
        fields.append(&open);
        fields.append(&gtk::Spinner::builder().spinning(true).build());
        dialog.set_extra_child(Some(&fields));
        dialog.add_responses(&[("cancel", t!("_Cancel"))]);
        dialog.set_close_response("cancel");
        let cancel = Arc::new(AtomicBool::new(false));
        if let Some(old) = self.sync_cancel.replace(Some(cancel.clone())) {
            old.store(true, Ordering::Relaxed);
        }
        let c = cancel.clone();
        dialog.connect_response(None, move |_, _| c.store(true, Ordering::Relaxed));
        dialog.present(Some(&self.win));
        let result = gio::spawn_blocking(move || github::device_finish(&code, cancel)).await;
        dialog.force_close();
        match result {
            Ok(Ok(token)) => Ok(Zeroizing::new(token)),
            Ok(Err(github::Error::Cancelled)) => Err(t!("Sign-in cancelled").into()),
            Ok(Err(e)) => Err(e.to_string()),
            Err(_) => Err(t!("Sign-in failed").into()),
        }
    }

    /// Signs out: forgets token, repository, key and sync state.
    fn github_sign_out(self: &Rc<Self>) {
        self.github_disconnect();
        github::secret::clear(github::secret::TOKEN);
        self.settings.borrow_mut().github_account.clear();
        self.persist_all();
    }

    /// Sets up a repository: checks it, asks for the sync password (a new
    /// one for a repository without QuickNotes data), keeps the key.
    fn github_connect(self: &Rc<Self>, input: String, done: Box<dyn Fn(Result<String, String>)>) {
        let Some(repo) = github::parse_repo(&input) else {
            done(Err(t!("Enter the repository as owner/name").into()));
            return;
        };
        let Some(token) = github::secret::load(github::secret::TOKEN) else {
            done(Err(t!("Sign in first").into()));
            return;
        };
        let this = self.clone();
        glib::spawn_future_local(async move {
            let (t, r) = (token.clone(), repo.clone());
            let info = match gio::spawn_blocking(move || github::inspect(&t, &r)).await {
                Ok(Ok(info)) => info,
                Ok(Err(e)) => return done(Err(e.to_string())),
                Err(_) => return done(Err(t!("Sign-in failed").into())),
            };
            let key = match info.vault.clone() {
                Some(lock) => {
                    let body = t!("Enter the sync password of {} – the one chosen when it was first used with QuickNotes.", info.repo);
                    match this.verify_password(lock, t!("Sync Password"), &body, t!("_Connect")).await {
                        Some(key) => key,
                        None => return done(Err(t!("Cancelled").into())),
                    }
                }
                None => {
                    let body = if info.private {
                        t!("Choose a sync password for {}. Your notes are encrypted with it before they are sent; other computers need it once to connect. A forgotten password cannot be recovered.", info.repo)
                    } else {
                        t!("{} is a public repository. Your notes are encrypted with the sync password before they are sent, but anyone can see how many there are and when they change. Choose a strong password – a forgotten one cannot be recovered.", info.repo)
                    };
                    let answer = this
                        .ask_password(PasswordPrompt {
                            heading: t!("Choose a Sync Password"),
                            body: &body,
                            confirm: true,
                            ok: t!("_Set Up"),
                            skip: None,
                            error: None,
                        })
                        .await;
                    let PasswordAnswer::Password(password) = answer else {
                        return done(Err(t!("Cancelled").into()));
                    };
                    let Ok((lock, key)) =
                        gio::spawn_blocking(move || crypto::Lock::create(&password, crypto::Kdf::new())).await
                    else {
                        return done(Err(t!("Cancelled").into()));
                    };
                    let (t, r) = (token.clone(), info.repo.clone());
                    match gio::spawn_blocking(move || github::create_vault(&t, &r, &lock)).await {
                        Ok(Ok(())) => key,
                        Ok(Err(e)) => return done(Err(e.to_string())),
                        Err(_) => return done(Err(t!("Sign-in failed").into())),
                    }
                }
            };
            if let Err(e) = github::secret::store(&github::secret::key_name(&info.repo), &key.export()) {
                return done(Err(t!("Cannot store the key in the keyring: {}", e)));
            }
            this.sync_disconnect();
            {
                let mut s = this.settings.borrow_mut();
                s.github_repo = info.repo.clone();
                s.github_sync = true;
            }
            this.persist_all();
            done(Ok(if info.private {
                t!("{} (private repository)", info.repo)
            } else {
                t!("{} (public repository, encrypted)", info.repo)
            }));
            this.github_sync_now();
        });
    }

    /// Stops syncing with the repository (its data stays on GitHub).
    fn github_disconnect(self: &Rc<Self>) {
        let repo = std::mem::take(&mut self.settings.borrow_mut().github_repo);
        if !repo.is_empty() {
            github::secret::clear(&github::secret::key_name(&repo));
        }
        github::clear_state();
        self.settings.borrow_mut().github_sync = false;
        self.persist_all();
    }

    /// Asks for the sync password of a target that is set up already, or
    /// for a new one: the data key, and the new wrapped key to store.
    async fn sync_key(self: &Rc<Self>, place: &str, vault: Option<crypto::Lock>) -> Result<(crypto::Key, Option<crypto::Lock>), String> {
        match vault {
            Some(lock) => {
                let body = t!("Enter the sync password of {} – the one chosen when it was first used with QuickNotes.", place);
                match self.verify_password(lock, t!("Sync Password"), &body, t!("_Connect")).await {
                    Some(key) => Ok((key, None)),
                    None => Err(t!("Cancelled").into()),
                }
            }
            None => {
                let body = t!("Choose a sync password for {}. Your notes are encrypted with it before they are sent; other computers need it once to connect. A forgotten password cannot be recovered.", place);
                let answer = self
                    .ask_password(PasswordPrompt {
                        heading: t!("Choose a Sync Password"),
                        body: &body,
                        confirm: true,
                        ok: t!("_Set Up"),
                        skip: None,
                        error: None,
                    })
                    .await;
                let PasswordAnswer::Password(password) = answer else {
                    return Err(t!("Cancelled").into());
                };
                match gio::spawn_blocking(move || crypto::Lock::create(&password, crypto::Kdf::new())).await {
                    Ok((lock, key)) => Ok((key, Some(lock))),
                    Err(_) => Err(t!("Cancelled").into()),
                }
            }
        }
    }

    /// Connects a sync target other than GitHub; `done` gets a description
    /// or an error. GitHub sync stops (one target at a time).
    fn sync_connect(self: &Rc<Self>, target: crate::remote::Target, secret: String, done: Box<dyn Fn(Result<String, String>)>) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let (t, s) = (target.clone(), secret.clone());
            let vault = match gio::spawn_blocking(move || crate::remote::prepare(&t, &s)).await {
                Ok(Ok(vault)) => vault,
                Ok(Err(e)) => return done(Err(e.to_string())),
                Err(_) => return done(Err(t!("Cancelled").into())),
            };
            let (key, new_lock) = match this.sync_key(&target.describe(), vault).await {
                Ok(k) => k,
                Err(e) => return done(Err(e)),
            };
            if let Some(lock) = new_lock {
                let (t, s) = (target.clone(), secret.clone());
                let written = gio::spawn_blocking(move || t.open_with(&s).and_then(|r| github::write_vault(&*r, &lock))).await;
                match written {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => return done(Err(e.to_string())),
                    Err(_) => return done(Err(t!("Cancelled").into())),
                }
            }
            if let Some(name) = target.secret_name() {
                if let Err(e) = github::secret::store(&name, &secret) {
                    return done(Err(t!("Cannot store the key in the keyring: {}", e)));
                }
            }
            if let Err(e) = github::secret::store(&github::secret::key_name(&target.id()), &key.export()) {
                return done(Err(t!("Cannot store the key in the keyring: {}", e)));
            }
            this.github_disconnect();
            this.sync_disconnect();
            {
                let mut s = this.settings.borrow_mut();
                s.sync_target = Some(target.clone());
                s.github_sync = true;
            }
            this.persist_all();
            done(Ok(format!("{}: {}", target.name(), target.describe())));
            this.github_sync_now();
        });
    }

    /// Signs in to Nextcloud in the browser, then connects its folder.
    fn nextcloud_sign_in(self: &Rc<Self>, server: String, folder: String, done: Box<dyn Fn(Result<String, String>)>) {
        let server = crate::remote::normalize_server(&server);
        let folder = folder.trim().trim_matches('/').to_owned();
        let folder = if folder.is_empty() { "QuickNotes".to_owned() } else { folder };
        let cancel = Arc::new(AtomicBool::new(false));
        if let Some(old) = self.sync_login.replace(Some(cancel.clone())) {
            old.store(true, Ordering::Relaxed);
        }
        let this = self.clone();
        glib::spawn_future_local(async move {
            let s = server.clone();
            let flow = match gio::spawn_blocking(move || crate::remote::nextcloud_start(&s)).await {
                Ok(Ok(flow)) => flow,
                Ok(Err(e)) => return done(Err(e.to_string())),
                Err(_) => return done(Err(t!("Cancelled").into())),
            };
            gtk::UriLauncher::new(&flow.login_url).launch(Some(&this.win), None::<&gio::Cancellable>, |_| {});
            this.toast(t!("Confirm the sign-in in the browser"));
            let result = gio::spawn_blocking(move || crate::remote::nextcloud_finish(&flow, cancel)).await;
            let (server_url, user, password) = match result {
                Ok(Ok(r)) => r,
                Ok(Err(e)) => return done(Err(e.to_string())),
                Err(_) => return done(Err(t!("Cancelled").into())),
            };
            let server = if server_url.is_empty() { server } else { server_url.trim_end_matches('/').to_owned() };
            this.sync_connect(crate::remote::Target::Nextcloud { server, user, folder }, password, done);
        });
    }

    /// Stops syncing with the target other than GitHub (its data stays).
    fn sync_disconnect(self: &Rc<Self>) {
        let target = self.settings.borrow_mut().sync_target.take();
        if let Some(target) = target {
            if let Some(name) = target.secret_name() {
                github::secret::clear(&name);
            }
            github::secret::clear(&github::secret::key_name(&target.id()));
            github::clear_state();
            self.settings.borrow_mut().github_sync = false;
            self.persist_all();
        }
    }

    /// Removes a tab from bookkeeping and remembers it for t!("Reopen Closed Tab").
    fn forget(self: &Rc<Self>, tab: &Tab) {
        let id = tab.meta().id;
        if matches!(tab, Tab::Text(t) if t.transient.get()) {
            self.tabs.borrow_mut().remove(&id);
            self.dirty.borrow_mut().remove(&id);
            return;
        }
        if self.sync_discard.borrow_mut().remove(&id) {
            // Deleted elsewhere.
            self.tabs.borrow_mut().remove(&id);
            self.dirty.borrow_mut().remove(&id);
            self.tab_folders.borrow_mut().remove(&id);
            store::delete_note(&id);
            self.refresh_tree();
            self.schedule_persist();
            return;
        }
        // Templates are kept even without the notes sidebar.
        let template = self.tab_folders.borrow().get(&id).is_some_and(|f| f == TEMPLATES_FOLDER);
        if features::on(Feature::Folders) || template {
            self.forget_into_library(tab);
            return;
        }
        let meta = tab.meta();
        let id = meta.id.clone();
        self.tabs.borrow_mut().remove(&id);
        self.dirty.borrow_mut().remove(&id);
        store::delete_note(&id);

        match tab {
            Tab::Text(t) if t.lock().is_some() => {
                // Kept sealed for *Reopen Closed Tab*.
                if let Some(sealed) = t.sealed_content() {
                    self.sealed_tabs.borrow_mut().insert(id, sealed);
                    self.closed.borrow_mut().push((TabMeta { lock: t.lock(), ..meta }, None));
                }
            }
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
            Some((meta, _)) if self.library.borrow().iter().any(|m| m.id == meta.id) => self.open_note(&meta.id),
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
        *self.folders.borrow_mut() = session.folders;
        *self.library.borrow_mut() = session.library;
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
        if let Some(folder) = &meta.folder {
            self.tab_folders.borrow_mut().insert(meta.id.clone(), folder.clone());
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
        *t.schema.borrow_mut() = meta.schema.clone();
        let linked = meta.template.is_some();
        *t.template.borrow_mut() = meta.template.clone();
        t.language_locked.set(meta.language_locked);
        t.split.set(meta.split);
        t.set_wrap(meta.wrap);
        t.set_language_id(meta.language.as_deref());
        if let Some(p) = &meta.path {
            t.set_large(self.is_large(p));
        }
        if let Some(lock) = meta.lock.clone() {
            let sealed = self.sealed_tabs.borrow_mut().remove(&meta.id).or_else(|| store::read_locked(&meta.id));
            let Some(sealed) = sealed else {
                self.toast(&t!("The protected note “{}” is missing", meta.title.as_deref().unwrap_or(&meta.id)));
                return None;
            };
            t.set_locked(lock, sealed);
            let page = self.add_tab(Tab::Text(t.clone()), false);
            self.update_lock_icon(&t);
            self.refresh_title(&t.id);
            t.set_preview(meta.preview);
            return Some(page);
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
        if linked {
            self.schedule_template_refresh();
        }
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
            if t.loading.get() || t.transient.get() {
                continue;
            }
            if let Some(sealed) = t.sealed_content() {
                if let Err(e) = store::write_locked(&id, &sealed) {
                    self.toast(&t!("Cannot store note: {}", e));
                }
            } else if t.needs_note_file() {
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
        if detected == Some("plantuml") && features::on(Feature::PlantUml) {
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
                if matches!(&e.tab, Tab::Text(t) if t.transient.get()) {
                    continue;
                }
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
        let folders = self.tab_folders.borrow();
        for meta in &mut tabs {
            meta.folder = folders.get(&meta.id).cloned();
        }
        Session {
            version: 1,
            active,
            tabs,
            library: self.library.borrow().clone(),
            folders: self.folders.borrow().clone(),
            settings,
            vault: self.vault.borrow().clone(),
            sealed: None,
        }
    }

    // ===== backup =========================================================

    /// Current session and note contents, straight from the editors.
    fn backup_contents(&self) -> (Session, crate::backup::Contents) {
        // The app password belongs to this installation, not to the backup.
        let session = Session { vault: None, ..self.snapshot() };
        let mut contents = crate::backup::Contents::default();
        for t in self.text_tabs() {
            // Protected tabs stay encrypted with their own password.
            if let Some(sealed) = t.sealed_content() {
                contents.locked.push((t.id.clone(), sealed));
            } else if t.needs_note_file() {
                contents.notes.push((t.id.clone(), t.text()));
            }
        }
        for m in self.library.borrow().iter().filter(|m| m.path.is_none()) {
            if m.lock.is_some() {
                if let Some(sealed) = store::read_locked(&m.id) {
                    contents.locked.push((m.id.clone(), sealed));
                }
            } else if let Some(text) = store::read_note(&m.id) {
                contents.notes.push((m.id.clone(), text));
            }
        }
        contents.learning = Some(self.shared.learning.borrow().to_json());
        (session, contents)
    }

    /// Asks for a password in a dialog (twice when `confirm`).
    async fn ask_password(&self, prompt: PasswordPrompt<'_>) -> PasswordAnswer {
        let dialog = adw::AlertDialog::new(Some(prompt.heading), Some(prompt.body));
        let fields = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let entry = |placeholder: &str| {
            gtk::PasswordEntry::builder()
                .show_peek_icon(true)
                .placeholder_text(placeholder)
                .activates_default(true)
                .build()
        };
        let first = entry(t!("Password"));
        fields.append(&first);
        let second = prompt.confirm.then(|| entry(t!("Repeat password")));
        if let Some(e) = &second {
            fields.append(e);
        }
        let hint = gtk::Label::builder().xalign(0.0).wrap(true).build();
        hint.add_css_class("caption");
        if let Some(error) = prompt.error {
            hint.set_text(error);
            hint.add_css_class("error");
        }
        fields.append(&hint);
        dialog.set_extra_child(Some(&fields));
        dialog.add_response("cancel", t!("_Cancel"));
        if let Some(skip) = prompt.skip {
            dialog.add_response("skip", skip);
        }
        dialog.add_response("ok", prompt.ok);
        dialog.set_response_appearance("ok", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("ok"));
        dialog.set_close_response("cancel");

        let update = {
            let (dialog, first, second, hint) = (dialog.clone(), first.clone(), second.clone(), hint.clone());
            let has_error = prompt.error.is_some();
            move || {
                let text = first.text();
                let (ok, message) = match &second {
                    None => (!text.is_empty(), None),
                    Some(_) if text.chars().count() < MIN_PASSWORD => {
                        (false, Some(t!("At least {} characters", MIN_PASSWORD)))
                    }
                    Some(s) if s.text() != text => (false, Some(t!("The passwords differ").to_owned())),
                    Some(_) => (true, Some(String::new())),
                };
                dialog.set_response_enabled("ok", ok);
                if let Some(m) = message {
                    hint.remove_css_class("error");
                    hint.set_text(&m);
                } else if has_error && !text.is_empty() {
                    hint.set_text("");
                }
            }
        };
        update();
        let u = update.clone();
        first.connect_changed(move |_| u());
        if let Some(s) = &second {
            let u = update.clone();
            s.connect_changed(move |_| u());
        }
        dialog.set_focus(Some(&first));
        let response = dialog.choose_future(&self.win).await;
        let answer = match response.as_str() {
            "ok" => PasswordAnswer::Password(Zeroizing::new(first.text().to_string())),
            "skip" => PasswordAnswer::Skip,
            _ => PasswordAnswer::Cancel,
        };
        first.set_text("");
        if let Some(s) = &second {
            s.set_text("");
        }
        answer
    }

    /// The app's data key while the app password is on.
    fn app_key(&self) -> Option<crypto::Key> {
        self.vault.borrow().as_ref().and(store::app_key())
    }

    // ===== app password ===================================================

    /// Asks for the current app password; the data key if it is right.
    async fn verify_app_password(&self, heading: &str, body: &str, ok: &str) -> Option<crypto::Key> {
        let lock = self.vault.borrow().clone()?;
        self.verify_password(lock, heading, body, ok).await
    }

    /// Asks for the password of `lock` until it is right; its key, or `None` when cancelled.
    async fn verify_password(&self, lock: crypto::Lock, heading: &str, body: &str, ok: &str) -> Option<crypto::Key> {
        let mut error = None;
        loop {
            let answer = self
                .ask_password(PasswordPrompt { heading, body, confirm: false, ok, skip: None, error })
                .await;
            let PasswordAnswer::Password(password) = answer else { return None };
            let lock = lock.clone();
            match gio::spawn_blocking(move || lock.open(&password)).await {
                Ok(Ok(key)) => return Some(key),
                _ => error = Some(t!("Wrong password – please try again")),
            }
        }
    }

    fn set_app_password(self: &Rc<Self>, done: Box<dyn Fn(bool)>) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let answer = this
                .ask_password(PasswordPrompt {
                    heading: t!("Set App Password"),
                    body: t!("QuickNotes will ask for this password at every start and keep your tabs and notes encrypted on disk. If you forget it, your notes cannot be recovered."),
                    confirm: true,
                    ok: t!("_Set Password"),
                    skip: None,
                    error: None,
                })
                .await;
            let PasswordAnswer::Password(password) = answer else {
                done(false);
                return;
            };
            let Ok((lock, key)) = gio::spawn_blocking(move || crypto::Lock::create(&password, crypto::Kdf::new())).await
            else {
                done(false);
                return;
            };
            this.persist_all();
            store::set_app_key(Some(key.clone()));
            *this.vault.borrow_mut() = Some(lock);
            let failed = store::convert_notes(None) + this.reseal_auto_backups(None, Some(&key));
            let _ = this.shared.learning.borrow().save();
            this.persist_all();
            this.menu_context.set(None);
            this.sync_ui();
            if failed > 0 {
                this.toast(&t!("App password set, but {} files could not be encrypted", failed));
            } else {
                this.toast(t!("App password set – your notes are now stored encrypted"));
            }
            done(true);
        });
    }

    fn change_app_password(self: &Rc<Self>, done: Box<dyn Fn(bool)>) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let Some(key) = this
                .verify_app_password(t!("Change App Password"), t!("Enter the current app password."), t!("_Next"))
                .await
            else {
                done(true);
                return;
            };
            let answer = this
                .ask_password(PasswordPrompt {
                    heading: t!("Change App Password"),
                    body: t!("Enter the new app password."),
                    confirm: true,
                    ok: t!("_Change Password"),
                    skip: None,
                    error: None,
                })
                .await;
            if let PasswordAnswer::Password(password) = answer {
                // Only the wrapped data key changes; the notes stay as they are.
                if let Ok(lock) =
                    gio::spawn_blocking(move || crypto::Lock::rewrap(&key, &password, crypto::Kdf::new())).await
                {
                    *this.vault.borrow_mut() = Some(lock);
                    this.persist_all();
                    this.toast(t!("App password changed"));
                }
            }
            done(true);
        });
    }

    fn remove_app_password(self: &Rc<Self>, done: Box<dyn Fn(bool)>) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let Some(key) = this
                .verify_app_password(
                    t!("Remove App Password"),
                    t!("Enter the app password to remove it. Your tabs and notes are then stored unencrypted again."),
                    t!("_Remove Password"),
                )
                .await
            else {
                done(true);
                return;
            };
            this.persist_all();
            store::set_app_key(None);
            *this.vault.borrow_mut() = None;
            let failed = store::convert_notes(Some(&key)) + this.reseal_auto_backups(Some(&key), None);
            let _ = this.shared.learning.borrow().save();
            this.persist_all();
            this.menu_context.set(None);
            this.sync_ui();
            if failed > 0 {
                this.toast(&t!("App password removed, but {} files could not be decrypted", failed));
            } else {
                this.toast(t!("App password removed"));
            }
            done(false);
        });
    }

    /// Re-encrypts the automatic backups (made before restores) for a new
    /// app key, or stores them unencrypted. Returns the number of failures.
    fn reseal_auto_backups(&self, old: Option<&crypto::Key>, new: Option<&crypto::Key>) -> usize {
        let dir = store::data_dir().join("backups");
        let Ok(entries) = std::fs::read_dir(&dir) else { return 0 };
        let mut failed = 0;
        for path in entries.flatten().map(|e| e.path()) {
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let Some(stem) = name.strip_suffix(".tar.gz").or_else(|| name.strip_suffix(".qnbackup")) else { continue };
            let Ok(backup) = crate::backup::read(&path, None, old) else {
                failed += 1;
                continue;
            };
            let notes = crate::backup::Contents::from(&backup);
            let seal = new.cloned().map(crate::backup::Seal::AppKey);
            let target = dir.join(format!("{stem}.{}", if seal.is_some() { "qnbackup" } else { "tar.gz" }));
            match crate::backup::write(&target, &backup.manifest.created, &backup.session, &notes, seal.as_ref()) {
                Ok(()) if target != path => {
                    let _ = std::fs::remove_file(&path);
                }
                Ok(()) => {}
                Err(_) => failed += 1,
            }
        }
        failed
    }

    // ===== tab password ===================================================

    /// Text tab the tab context menu is open for, or the current one.
    fn menu_text_tab(&self) -> Option<Rc<EditorTab>> {
        match self.menu_page().and_then(|p| self.entry_for_page(&p)).map(|e| e.tab) {
            Some(Tab::Text(t)) => Some(t),
            _ => None,
        }
    }

    fn update_lock_actions(&self) {
        let tab = self.menu_text_tab();
        let state = tab.as_ref().map(|t| t.lock_state());
        let note = tab.as_ref().is_some_and(|t| t.path.borrow().is_none());
        self.set_enabled("tab-protect", note && state == Some(LockState::Open));
        for name in ["tab-lock", "tab-change-password", "tab-remove-password"] {
            self.set_enabled(name, state == Some(LockState::Unlocked));
        }
    }

    /// Closed or open lock on the tab.
    fn update_lock_icon(&self, t: &EditorTab) {
        let Some(page) = self.tabs.borrow().get(&t.id).map(|e| e.page.clone()) else { return };
        let (icon, tip) = match t.lock_state() {
            LockState::Open => (None, ""),
            LockState::Unlocked => (Some("changes-allow-symbolic"), t!("Protected with a password, unlocked")),
            LockState::Locked => (Some("changes-prevent-symbolic"), t!("Protected with a password, locked")),
        };
        page.set_icon(icon.map(gio::ThemedIcon::new).as_ref());
        page.set_tooltip(tip);
    }

    fn protect_tab(self: &Rc<Self>, t: Rc<EditorTab>) {
        if t.path.borrow().is_some() {
            self.toast(t!("Only notes (tabs without a file) can be protected with a password"));
            return;
        }
        let this = self.clone();
        glib::spawn_future_local(async move {
            let answer = this
                .ask_password(PasswordPrompt {
                    heading: t!("Protect Tab with Password"),
                    body: t!("The note is then stored encrypted with this password, independent of the app password. If you forget it, the note cannot be recovered."),
                    confirm: true,
                    ok: t!("_Protect"),
                    skip: None,
                    error: None,
                })
                .await;
            let PasswordAnswer::Password(password) = answer else { return };
            if let Ok((lock, key)) =
                gio::spawn_blocking(move || crypto::Lock::create(&password, crypto::Kdf::new())).await
            {
                t.protect(lock, key);
                this.toast(t!("Tab protected – it locks with “Lock Tab” and when QuickNotes closes"));
            }
        });
    }

    fn change_tab_password(self: &Rc<Self>, t: Rc<EditorTab>) {
        let (Some(lock), Some(key)) = (t.lock(), t.tab_key()) else { return };
        let this = self.clone();
        glib::spawn_future_local(async move {
            let heading = t!("Change Tab Password");
            if this.verify_password(lock, heading, t!("Enter the current password of this tab."), t!("_Next")).await.is_none() {
                return;
            }
            let answer = this
                .ask_password(PasswordPrompt {
                    heading,
                    body: t!("Enter the new password of this tab."),
                    confirm: true,
                    ok: t!("_Change Password"),
                    skip: None,
                    error: None,
                })
                .await;
            let PasswordAnswer::Password(password) = answer else { return };
            if let Ok(lock) = gio::spawn_blocking(move || crypto::Lock::rewrap(&key, &password, crypto::Kdf::new())).await {
                t.set_lock(lock);
                this.toast(t!("Tab password changed"));
            }
        });
    }

    fn remove_tab_password(self: &Rc<Self>, t: Rc<EditorTab>) {
        let Some(lock) = t.lock() else { return };
        let this = self.clone();
        glib::spawn_future_local(async move {
            let ok = this
                .verify_password(
                    lock,
                    t!("Remove Tab Password"),
                    t!("Enter the password of this tab to remove it. The note is then stored like any other note."),
                    t!("_Remove Password"),
                )
                .await;
            if ok.is_some() {
                t.unprotect();
                this.toast(t!("Tab password removed"));
            }
        });
    }

    /// Saves everything and shows the unlock window instead of the notes.
    fn lock_app(self: &Rc<Self>) {
        if self.vault.borrow().is_none() {
            return;
        }
        let Some(app) = self.win.application().and_downcast::<adw::Application>() else { return };
        self.persist_all();
        if let Some(running) = self.presentation.borrow_mut().take() {
            running.end();
        }
        if let Some(study) = self.study.borrow_mut().take() {
            study.close();
        }
        if let Some((win, _)) = self.cards_overview.borrow_mut().take() {
            win.close();
        }
        show_unlock(&app, store::load_session(), Vec::new());
        self.locking.set(true);
        self.win.close();
        store::set_app_key(None);
    }

    fn backup(self: &Rc<Self>) {
        self.persist_all();
        let this = self.clone();
        glib::spawn_future_local(async move {
            let answer = this
                .ask_password(PasswordPrompt {
                    heading: t!("Back Up Notes"),
                    body: if this.vault.borrow().is_some() {
                        t!("Encrypt the backup with a password? Your notes are protected by the app password – without a password they would be stored readable in the backup. A forgotten password cannot be recovered.")
                    } else {
                        t!("Encrypt the backup with a password? It can then only be restored with this password – a forgotten password cannot be recovered.")
                    },
                    confirm: true,
                    ok: t!("_Encrypt"),
                    skip: Some(t!("_Without Password")),
                    error: None,
                })
                .await;
            let password = match answer {
                PasswordAnswer::Cancel => return,
                PasswordAnswer::Skip => None,
                PasswordAnswer::Password(p) => Some(p),
            };
            let date = local_time("%Y-%m-%d").unwrap_or_else(|| "backup".into());
            let ext = if password.is_some() { "qnbackup" } else { "tar.gz" };
            let dialog = gtk::FileDialog::builder()
                .title(t!("Back Up Notes"))
                .modal(true)
                .initial_name(format!("quicknotes-backup-{date}.{ext}"))
                .build();
            if let Some(home) = dirs::home_dir() {
                dialog.set_initial_folder(Some(&gio::File::for_path(home)));
            }
            let Ok(file) = dialog.save_future(Some(&this.win)).await else { return };
            let Some(path) = file.path() else { return };
            let (session, notes) = this.backup_contents();
            let count = session.tabs.len();
            let created = iso_now();
            let target = path.clone();
            let encrypted = password.is_some();
            let result = gio::spawn_blocking(move || {
                let seal = match password {
                    Some(pw) => {
                        let kdf = crypto::Kdf::new();
                        let key = kdf.derive(&pw).map_err(|e| e.to_string())?;
                        Some(crate::backup::Seal::Password { key, kdf })
                    }
                    None => None,
                };
                crate::backup::write(&target, &created, &session, &notes, seal.as_ref())
            })
            .await
            .unwrap_or_else(|_| Err("backup failed".into()));
            match result {
                Ok(()) if encrypted => {
                    this.toast(&t!("Encrypted backup of {} saved to {}", tabs_count(count), tilde(&path)))
                }
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
        filter.add_pattern("*.qnbackup");
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
            let Some(backup) = this.read_backup(path).await else { return };
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

    /// Reads a backup, asking for the password of encrypted ones.
    async fn read_backup(&self, path: PathBuf) -> Option<crate::backup::Backup> {
        use crate::backup::ReadError;
        let app_key = self.app_key();
        let mut password: Option<Zeroizing<String>> = None;
        loop {
            let (p, pw, key) = (path.clone(), password.clone(), app_key.clone());
            let result = gio::spawn_blocking(move || crate::backup::read(&p, pw.as_deref().map(String::as_str), key.as_ref()))
                .await
                .unwrap_or_else(|_| Err(ReadError::Failed("cannot read the backup".into())));
            let error = match result {
                Ok(b) => return Some(b),
                Err(ReadError::NeedsPassword) => None,
                Err(ReadError::WrongPassword) if password.is_some() => Some(t!("Wrong password – please try again")),
                Err(ReadError::WrongPassword) => {
                    self.toast(t!("This automatic backup was encrypted with an earlier app password"));
                    return None;
                }
                Err(e) => {
                    self.toast(&t!("Restore failed: {}", e));
                    return None;
                }
            };
            let answer = self
                .ask_password(PasswordPrompt {
                    heading: t!("Encrypted Backup"),
                    body: t!("Enter the password of this backup."),
                    confirm: false,
                    ok: t!("_Decrypt"),
                    skip: None,
                    error,
                })
                .await;
            match answer {
                PasswordAnswer::Password(p) => password = Some(p),
                _ => return None,
            }
        }
    }

    /// Restores the tabs of a backup. Returns (restored, missing files).
    fn import_tabs(self: &Rc<Self>, backup: &crate::backup::Backup, merge: bool) -> (usize, usize) {
        let (mut restored, mut missing) = (0, 0);
        for meta in &backup.session.tabs {
            let content = backup.notes.get(&meta.id).cloned();
            if meta.lock.is_some() {
                let Some(sealed) = backup.locked.get(&meta.id) else { continue };
                let mut meta = meta.clone();
                if merge && self.tabs.borrow().contains_key(&meta.id) {
                    meta.id = store::new_id();
                }
                self.sealed_tabs.borrow_mut().insert(meta.id.clone(), sealed.clone());
                let id = meta.id.clone();
                if self.restore_tab(meta, None).is_some() {
                    restored += 1;
                    self.mark_dirty(&id);
                }
                continue;
            }
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
        // The notes library: closed notes and folders.
        for meta in &backup.session.library {
            let mut meta = meta.clone();
            let original = meta.id.clone();
            let taken = |id: &str| self.tabs.borrow().contains_key(id) || self.library.borrow().iter().any(|m| m.id == id);
            if merge && taken(&meta.id) {
                meta.id = store::new_id();
            }
            if meta.path.is_none() {
                let stored = match (&meta.lock, backup.locked.get(&original), backup.notes.get(&original)) {
                    (Some(_), Some(sealed), _) => store::write_locked(&meta.id, sealed),
                    (None, _, Some(text)) => store::write_note(&meta.id, text),
                    _ => continue,
                };
                if let Err(e) = stored {
                    self.toast(&t!("Cannot store note: {}", e));
                    continue;
                }
            }
            self.library.borrow_mut().push(meta);
            restored += 1;
        }
        {
            let mut folders = self.folders.borrow_mut();
            if merge {
                for f in &backup.session.folders {
                    if !folders.iter().any(|g| g.id == f.id) {
                        folders.push(f.clone());
                    }
                }
            } else {
                *folders = backup.session.folders.clone();
            }
        }
        self.refresh_tree();
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
        if let Some(data) = &backup.learning {
            let mut store = self.shared.learning.borrow_mut();
            store.merge(crate::learning::Store::from_backup(data));
            let _ = store.save();
        }
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
        // Encrypted with the app key while the app password is on.
        let seal = self.app_key().map(crate::backup::Seal::AppKey);
        let ext = if seal.is_some() { "qnbackup" } else { "tar.gz" };
        let safety = dir.join(format!("before-restore-{stamp}.{ext}"));
        let (session, notes) = self.backup_contents();
        if let Err(e) = std::fs::create_dir_all(&dir)
            .map_err(|e| e.to_string())
            .and_then(|_| crate::backup::write(&safety, &iso_now(), &session, &notes, seal.as_ref()))
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
        for m in self.library.borrow_mut().drain(..) {
            if m.path.is_none() {
                store::delete_note(&m.id);
            }
        }
        self.folders.borrow_mut().clear();
        self.tab_folders.borrow_mut().clear();

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
        self.set_format_bar(restored.format_bar);
        self.apply_font();
        self.apply_tab_colors();
        self.sync_layout();

        if let Some(data) = &backup.learning {
            let mut store = self.shared.learning.borrow_mut();
            *store = crate::learning::Store::from_backup(data);
            let _ = store.save();
        }
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
            t.lint();
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
        if t.preview_kind() == Some(PreviewKind::Mindmap) {
            self.export_mindmap(&t, format);
            return;
        }
        if format == ExportFormat::Svg {
            return;
        }
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
            ExportFormat::Pdf | ExportFormat::Svg => "pdf",
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
                ExportFormat::Pdf | ExportFormat::Svg => {
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

    /// Exports a mind map as SVG, or as PDF on an A4 page turned to fit.
    fn export_mindmap(self: &Rc<Self>, t: &Rc<EditorTab>, format: ExportFormat) {
        let map = t.mindmap();
        let ext = if format == ExportFormat::Svg { "svg" } else { "pdf" };
        let stem = t
            .path
            .borrow()
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().trim_end_matches(".md").trim_end_matches(".mindmap").to_owned())
            .unwrap_or_else(|| file_name_for(&t.title()));
        let dialog = gtk::FileDialog::builder()
            .title(t!("Export as {}", ext.to_uppercase()))
            .modal(true)
            .initial_name(format!("{stem}.{ext}"))
            .build();
        if let Some(dir) = t.base_dir() {
            dialog.set_initial_folder(Some(&gio::File::for_path(dir)));
        }
        let page = crate::mindmap::export_page(&map);
        let this = self.clone();
        glib::spawn_future_local(async move {
            let Ok(file) = dialog.save_future(Some(&this.win)).await else { return };
            let Some(path) = file.path() else { return };
            this.toast(&t!("Exporting {}…", ext.to_uppercase()));
            let weak = this.weak();
            crate::export::run_script(page, None, crate::mindmap::EXPORT_SCRIPT.to_owned(), move |result| {
                let Some(this) = weak.upgrade() else { return };
                #[derive(serde::Deserialize)]
                struct Drawn {
                    svg: String,
                    width: f64,
                    height: f64,
                    background: String,
                }
                let drawn = result.and_then(|json| {
                    // The script result is a JSON string holding the JSON object.
                    let inner: String = serde_json::from_str(&json).unwrap_or(json);
                    serde_json::from_str::<Drawn>(&inner).map_err(|e| e.to_string())
                });
                let drawn = match drawn {
                    Ok(d) => d,
                    Err(e) => {
                        this.export_done(&path, Err(e));
                        return;
                    }
                };
                if format == ExportFormat::Svg {
                    let svg = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}\n", drawn.svg);
                    let result = std::fs::write(&path, svg).map_err(|e| e.to_string());
                    this.export_done(&path, result);
                    return;
                }
                let landscape = drawn.width > drawn.height;
                let (width_mm, height_mm) = if landscape { (297.0, 210.0) } else { (210.0, 297.0) };
                let html = crate::mindmap::pdf_page(&drawn.svg, &drawn.background, width_mm, height_mm);
                let weak = this.weak();
                let target = path.clone();
                crate::export::pdf(html, None, path, crate::export::Page::Custom { width_mm, height_mm }, move |result| {
                    let Some(this) = weak.upgrade() else { return };
                    if result.is_err() {
                        this.export_done(&target, result);
                        return;
                    }
                    // The printer may still be writing: shrink once the file is complete.
                    let path = target.clone();
                    glib::spawn_future_local(async move {
                        let mut last = 0;
                        for _ in 0..50 {
                            glib::timeout_future(std::time::Duration::from_millis(100)).await;
                            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                            if size > 0 && size == last {
                                break;
                            }
                            last = size;
                        }
                        let p = path.clone();
                        let shrunk = gio::spawn_blocking(move || crate::export::dedup_pdf(&p)).await;
                        if let Ok(Err(e)) = shrunk {
                            eprintln!("quicknotes: PDF not shrunk: {e}");
                        }
                        this.export_done(&path, Ok(()));
                    });
                });
            });
        });
    }

    /// Exports a presentation as stand-alone HTML file (with keyboard
    /// navigation and animations) or as PDF with one slide per page.
    fn export_presentation(self: &Rc<Self>, t: &Rc<EditorTab>, deck: slides::Deck, format: ExportFormat) {
        let ext = match format {
            ExportFormat::Html => "html",
            ExportFormat::Pdf | ExportFormat::Svg => "pdf",
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
            ExportFormat::Pdf | ExportFormat::Svg => slides::Mode::Print,
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
                ExportFormat::Pdf | ExportFormat::Svg => {
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

    /// File name stem for exports of a tab.
    fn export_stem(t: &EditorTab, title: &str) -> String {
        t.path
            .borrow()
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().trim_end_matches(".md").trim_end_matches(".slides").to_owned())
            .map(|s| s.rsplit_once('.').map_or(s.clone(), |(stem, _)| stem.to_owned()))
            .unwrap_or_else(|| file_name_for(title))
    }

    /// Exports the document as image(s): SVG, PNG or JPG; documents as one
    /// tall image or one per page, presentations one per slide.
    fn export_image(self: &Rc<Self>) {
        let Some(t) = self.current_text() else { return };
        let deck = t.deck();
        let (document, title, base_uri) = match &deck {
            Some(deck) => {
                let page = slides::page(slides::Mode::Print, Some(deck), slides::PageOptions::default());
                let title = if deck.title.is_empty() { t.title() } else { deck.title.clone() };
                (page, title, t.slides_base_uri())
            }
            None => {
                let Some(body) = t.document_html() else {
                    self.toast(t!("Export works for Markdown and reStructuredText"));
                    return;
                };
                let title = html::first_heading(&body).unwrap_or_else(|| t.title());
                (html::standalone(&title, &body), title, t.base_uri())
            }
        };
        let slides_page = deck.as_ref().map(|d| d.page_mm());
        let stem = Self::export_stem(&t, &title);
        let base_dir = t.base_dir();
        let this = self.clone();
        glib::spawn_future_local(async move {
            let Some((format, tall)) = this.image_options(slides_page.is_some()).await else { return };
            let page = match (slides_page, tall) {
                (Some((width_mm, height_mm)), _) => crate::export::Page::Custom { width_mm, height_mm },
                (None, true) => crate::export::Page::Tall { factor: 1.3 },
                (None, false) => crate::export::Page::A4,
            };
            let ext = format.ext();
            // One file, or a folder for one image per page/slide.
            let target = if tall && slides_page.is_none() {
                let dialog = gtk::FileDialog::builder()
                    .title(t!("Export as {}", format.label()))
                    .modal(true)
                    .initial_name(format!("{stem}.{ext}"))
                    .build();
                if let Some(dir) = &base_dir {
                    dialog.set_initial_folder(Some(&gio::File::for_path(dir)));
                }
                let Ok(file) = dialog.save_future(Some(&this.win)).await else { return };
                let Some(path) = file.path() else { return };
                ImageTarget::File(path)
            } else {
                let dialog = gtk::FileDialog::builder().title(t!("Folder for the Images")).modal(true).build();
                if let Some(dir) = &base_dir {
                    dialog.set_initial_folder(Some(&gio::File::for_path(dir)));
                }
                let Ok(folder) = dialog.select_folder_future(Some(&this.win)).await else { return };
                let Some(path) = folder.path() else { return };
                ImageTarget::Folder(path, stem.clone())
            };
            this.toast(t!("Exporting images…"));
            let weak = this.weak();
            crate::export::images(document, base_uri, page, format, move |result| {
                let Some(this) = weak.upgrade() else { return };
                match result.and_then(|images| target.write(&images, ext)) {
                    Ok(path) => this.export_done(&path, Ok(())),
                    Err(e) => this.export_done(Path::new(""), Err(e)),
                }
            });
        });
    }

    /// Asks for format and (for documents) layout of an image export.
    /// Returns the format and whether the document becomes one tall image.
    async fn image_options(self: &Rc<Self>, slides: bool) -> Option<(crate::images::ImageFormat, bool)> {
        use crate::images::ImageFormat;
        let (last_format, last_layout) = self.image_export.get();
        let dialog = adw::AlertDialog::new(Some(t!("Export as Image")), None);
        let list = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).build();
        list.add_css_class("boxed-list");
        let labels: Vec<&str> = ImageFormat::ALL.iter().map(|f| f.label()).collect();
        let format = adw::ComboRow::builder().title(t!("File Format")).model(&gtk::StringList::new(&labels)).build();
        format.set_selected(last_format as u32);
        format.set_subtitle(t!("SVG stays sharp at any size; PNG and JPG are pictures"));
        list.append(&format);
        let layout = adw::ComboRow::builder()
            .title(t!("Pages"))
            .model(&gtk::StringList::new(&[t!("One tall image"), t!("One image per page (A4)")]))
            .build();
        layout.set_selected(last_layout as u32);
        if slides {
            dialog.set_body(t!("Each slide becomes an image."));
        } else {
            list.append(&layout);
        }
        dialog.set_extra_child(Some(&list));
        dialog.add_response("cancel", t!("_Cancel"));
        dialog.add_response("export", t!("_Export"));
        dialog.set_response_appearance("export", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("export"));
        dialog.set_close_response("cancel");
        if dialog.choose_future(&self.win).await != "export" {
            return None;
        }
        let (f, l) = (format.selected() as usize, layout.selected() as usize);
        self.image_export.set((f, l));
        Some((ImageFormat::ALL.get(f).copied().unwrap_or(ImageFormat::Png), l == 0))
    }

    /// Exports a Markdown or reStructuredText document as EPUB e-book.
    fn export_epub(self: &Rc<Self>) {
        let Some(t) = self.current_text() else { return };
        if t.deck().is_some() {
            self.toast(t!("EPUB export works for Markdown and reStructuredText"));
            return;
        }
        let Some(body) = t.document_html() else {
            self.toast(t!("EPUB export works for Markdown and reStructuredText"));
            return;
        };
        let title = html::first_heading(&body).unwrap_or_else(|| t.title());
        let meta = crate::epub::Meta::from_source(&t.text(), &title);
        let stem = Self::export_stem(&t, &title);
        let dialog = gtk::FileDialog::builder()
            .title(t!("Export as {}", "EPUB"))
            .modal(true)
            .initial_name(format!("{stem}.epub"))
            .build();
        let base_dir = t.base_dir();
        if let Some(dir) = &base_dir {
            dialog.set_initial_folder(Some(&gio::File::for_path(dir)));
        }
        let document = html::standalone(&meta.title, &body);
        let base_uri = t.base_uri();
        let this = self.clone();
        glib::spawn_future_local(async move {
            let Ok(file) = dialog.save_future(Some(&this.win)).await else { return };
            let Some(path) = file.path() else { return };
            this.toast(t!("Exporting EPUB…"));
            let weak = this.weak();
            let script = crate::epub::EXTRACT_JS.to_owned();
            crate::export::run_script(document, base_uri, script, move |result| {
                let Some(this) = weak.upgrade() else { return };
                let result = result
                    .and_then(|json| serde_json::from_str::<crate::epub::Extract>(&json).map_err(|e| e.to_string()))
                    .and_then(|extract| {
                        crate::epub::build(&meta, extract, base_dir.as_deref(), &html::ebook_css(), crate::notation::katex_css())
                    })
                    .and_then(|epub| std::fs::write(&path, epub).map_err(|e| e.to_string()));
                this.export_done(&path, result);
            });
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
                    t.lint();
                    this.sync_ui();
                }
                Err(e) => this.toast(&t!("Save failed: {}", e)),
            }
        });
    }

    /// Assigns a JSON Schema or XML Schema (URL or file) to the current note.
    fn assign_schema(self: &Rc<Self>) {
        let Some(t) = self.current_text().filter(|t| t.data_kind().is_some()) else { return };
        let entry = gtk::Entry::builder()
            .text(t.schema.borrow().clone().unwrap_or_default())
            .placeholder_text(t!("URL or file"))
            .activates_default(true)
            .hexpand(true)
            .build();
        let choose = gtk::Button::builder().icon_name("document-open-symbolic").tooltip_text(t!("Choose File…")).build();
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        row.add_css_class("linked");
        row.append(&entry);
        row.append(&choose);
        let current = gtk::Label::builder().wrap(true).xalign(0.0).build();
        current.add_css_class("dim-label");
        if let Some(label) = t.schema_label() {
            current.set_text(&format!("{}\n{}", label, t.schema_url().unwrap_or_default()));
        } else {
            current.set_text(t!("No schema in use. Without one, only the syntax is checked."));
        }
        let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
        content.append(&row);
        content.append(&current);
        let dialog = adw::AlertDialog::new(
            Some(t!("Assign Schema")),
            Some(t!("A JSON Schema for JSON and YAML, an XML Schema (XSD) for XML. An assigned schema replaces the one named in the document or found by the file name.")),
        );
        dialog.set_extra_child(Some(&content));
        let assigned = t.schema.borrow().is_some();
        if assigned {
            dialog.add_responses(&[("cancel", t!("_Cancel")), ("remove", t!("_Remove")), ("assign", t!("_Assign"))]);
            dialog.set_response_appearance("remove", adw::ResponseAppearance::Destructive);
        } else {
            dialog.add_responses(&[("cancel", t!("_Cancel")), ("assign", t!("_Assign"))]);
        }
        dialog.set_response_appearance("assign", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("assign"));
        dialog.set_close_response("cancel");
        let win = self.win.clone();
        let target = entry.clone();
        choose.connect_clicked(move |_| {
            let filters = gio::ListStore::new::<gtk::FileFilter>();
            let filter = gtk::FileFilter::new();
            filter.set_name(Some(t!("Schemas")));
            for pattern in ["*.json", "*.xsd", "*.yaml", "*.yml"] {
                filter.add_pattern(pattern);
            }
            filters.append(&filter);
            let all = gtk::FileFilter::new();
            all.set_name(Some(t!("All Files")));
            all.add_pattern("*");
            filters.append(&all);
            let picker = gtk::FileDialog::builder().title(t!("Choose Schema")).modal(true).filters(&filters).default_filter(&filter).build();
            let win = win.clone();
            let target = target.clone();
            glib::spawn_future_local(async move {
                if let Ok(file) = picker.open_future(Some(&win)).await {
                    if let Some(path) = file.path() {
                        target.set_text(&path.to_string_lossy());
                    }
                }
            });
        });
        let this = self.clone();
        glib::spawn_future_local(async move {
            let schema = match dialog.choose_future(&this.win).await.as_str() {
                "assign" => Some(entry.text().trim().to_owned()).filter(|s| !s.is_empty()),
                "remove" => None,
                _ => return,
            };
            *t.schema.borrow_mut() = schema;
            t.lint();
            this.mark_dirty(&t.id);
            this.update_problems_status();
        });
    }

    // ===== templates ======================================================

    /// The folder holding the templates (created when needed).
    fn ensure_templates_folder(self: &Rc<Self>) {
        let missing = !self.folders.borrow().iter().any(|f| f.id == TEMPLATES_FOLDER);
        if missing {
            self.folders.borrow_mut().push(Folder { id: TEMPLATES_FOLDER.into(), name: t!("Templates").into(), parent: None, collapsed: false });
            self.refresh_tree();
            self.schedule_persist();
        }
    }

    /// All templates: open ones and those in the library, by name.
    fn template_list(&self) -> Vec<TemplateNote> {
        let mut out = Vec::new();
        let folders = self.tab_folders.borrow();
        for (id, entry) in self.tabs.borrow().iter() {
            let Tab::Text(t) = &entry.tab else { continue };
            if folders.get(id).is_some_and(|f| f == TEMPLATES_FOLDER) && t.lock_state() == LockState::Open {
                out.push(TemplateNote { id: id.clone(), name: t.title(), language: t.language_id(), text: t.text() });
            }
        }
        for m in self.library.borrow().iter() {
            if m.folder.as_deref() == Some(TEMPLATES_FOLDER) && !m.trashed && m.lock.is_none() && m.path.is_none() {
                if let Some(text) = store::read_note(&m.id) {
                    out.push(TemplateNote { id: m.id.clone(), name: library_title(m), language: m.language.clone(), text });
                }
            }
        }
        out.sort_by_key(|t| t.name.to_lowercase());
        out
    }

    /// A new template (empty, or with the text and syntax of `from`).
    fn new_template(self: &Rc<Self>, from: Option<Rc<EditorTab>>) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let heading = if from.is_some() { t!("Save as Template") } else { t!("New Template") };
            let initial = from.as_ref().map(|t| t.title()).unwrap_or_default();
            let Some(name) = this.ask_folder_name(heading, &initial).await else { return };
            this.ensure_templates_folder();
            let t = this.new_note();
            *t.custom_title.borrow_mut() = Some(name);
            match &from {
                Some(src) => {
                    t.set_text_irreversible(&src.text());
                    t.set_language_id(src.language_id().as_deref());
                    t.language_locked.set(src.language_locked.get());
                }
                None => t.set_text_irreversible(&format!("{}\n", t!("{# Fields: {{ date }}, {{ time }}, {{ title }}, {{ ask \"Question\" }}, {{ cursor }} – building blocks: {% include \"Name\" %} – see Help (F1) #}"))),
            }
            t.place_cursor(CursorTarget::End);
            this.tab_folders.borrow_mut().insert(t.id.clone(), TEMPLATES_FOLDER.into());
            this.refresh_title(&t.id);
            this.refresh_tree();
            this.mark_dirty(&t.id);
            this.sync_ui();
            if from.is_some() {
                this.toast(t!("Saved as template"));
            }
        });
    }

    /// Lets the user pick a template to create a note from, insert or edit.
    fn pick_template(self: &Rc<Self>, usage: TemplateUse) {
        let list = self.template_list();
        let (heading, action) = match usage {
            TemplateUse::New => (t!("New from Template"), t!("_Create")),
            TemplateUse::Insert => (t!("Insert Template"), t!("_Insert")),
            TemplateUse::Edit => (t!("Edit Templates"), t!("_Open")),
        };
        let dialog = adw::AlertDialog::new(Some(heading), None);
        let content = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let search = gtk::SearchEntry::builder().placeholder_text(t!("Search templates")).build();
        let rows = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::Browse).build();
        rows.add_css_class("boxed-list");
        for tpl in &list {
            let row = adw::ActionRow::builder().title(glib::markup_escape_text(&tpl.name).as_str()).activatable(true).build();
            let what = tpl.language.as_deref().and_then(langs::by_id).map(|l| l.name().to_string()).unwrap_or_else(|| t!("Plain Text").into());
            row.set_subtitle(&what);
            rows.append(&row);
        }
        if let Some(first) = rows.row_at_index(0) {
            rows.select_row(Some(&first));
        }
        let names: Vec<String> = list.iter().map(|t| t.name.to_lowercase()).collect();
        {
            let rows2 = rows.clone();
            let names = names.clone();
            search.connect_search_changed(move |s| {
                let q = s.text().to_lowercase();
                let names = names.clone();
                rows2.set_filter_func(move |row| names.get(row.index() as usize).is_some_and(|n| n.contains(&q)));
                // Keep a visible row selected.
                let mut i = 0;
                while let Some(r) = rows2.row_at_index(i) {
                    if r.is_child_visible() {
                        rows2.select_row(Some(&r));
                        break;
                    }
                    i += 1;
                }
            });
        }
        let scroll = gtk::ScrolledWindow::builder()
            .child(&rows)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .min_content_height(220)
            .max_content_height(360)
            .propagate_natural_height(true)
            .build();
        if list.is_empty() {
            let empty = gtk::Label::builder()
                .label(t!("No templates yet. Create one with “New Template” or save a note with “Save as Template…”."))
                .wrap(true)
                .build();
            empty.add_css_class("dim-label");
            content.append(&empty);
        } else {
            content.append(&search);
            content.append(&scroll);
        }
        let new_btn = gtk::Button::builder().label(t!("New Template…")).halign(gtk::Align::Start).build();
        new_btn.add_css_class("flat");
        content.append(&new_btn);
        dialog.set_extra_child(Some(&content));
        let mut responses = vec![("cancel", t!("_Cancel"))];
        if usage == TemplateUse::Edit && !list.is_empty() {
            responses.push(("remove", t!("Remove from Templates")));
        } else if !list.is_empty() {
            responses.push(("edit", t!("_Edit")));
        }
        if !list.is_empty() {
            responses.push(("use", action));
        }
        dialog.add_responses(&responses);
        if responses.iter().any(|(r, _)| *r == "use") {
            dialog.set_response_appearance("use", adw::ResponseAppearance::Suggested);
            dialog.set_default_response(Some("use"));
        }
        dialog.set_close_response("cancel");
        let d = dialog.clone();
        let (rows3, names3) = (rows.clone(), names.clone());
        search.connect_activate(move |s| {
            // The filter follows the search text with a delay; pick the
            // first match of what was typed.
            let q = s.text().to_lowercase();
            if let Some(i) = names3.iter().position(|n| n.contains(&q)) {
                if let Some(r) = rows3.row_at_index(i as i32) {
                    rows3.select_row(Some(&r));
                }
            }
            d.emit_by_name::<()>("response", &[&"use"]);
            d.close();
        });
        let d = dialog.clone();
        rows.connect_row_activated(move |_, _| {
            d.emit_by_name::<()>("response", &[&"use"]);
            d.close();
        });
        let d = dialog.clone();
        let this = self.clone();
        new_btn.connect_clicked(move |_| {
            d.close();
            this.new_template(None);
        });
        // Typing goes to the search field (focus is only possible once shown).
        search.connect_map(|s| {
            s.grab_focus();
        });
        let this = self.clone();
        glib::spawn_future_local(async move {
            let response = dialog.choose_future(&this.win).await;
            let Some(tpl) = rows.selected_row().and_then(|r| list.get(r.index() as usize).cloned()) else { return };
            match (response.as_str(), usage) {
                ("edit", _) | ("use", TemplateUse::Edit) => this.open_note(&tpl.id),
                ("remove", _) => {
                    this.set_note_folder(&tpl.id, None);
                    this.refresh_tree();
                    this.schedule_persist();
                    this.toast(&t!("“{}” is no longer a template", tpl.name));
                }
                ("use", _) => this.apply_template(tpl, usage).await,
                _ => {}
            }
        });
    }

    /// Fills in a template (asking for its questions) and creates a note
    /// from it or inserts it at the cursor.
    async fn apply_template(self: &Rc<Self>, tpl: TemplateNote, usage: TemplateUse) {
        let target = self.current_text();
        let lib = TemplateLib(self.template_list());
        let clipboard = self.win.clipboard().read_text_future().await.ok().flatten().map(|s| s.to_string()).unwrap_or_default();
        let selection = match (&target, usage) {
            (Some(t), TemplateUse::Insert) => t.buffer.selection_bounds().map(|(s, e)| t.buffer.text(&s, &e, true).to_string()).unwrap_or_default(),
            _ => String::new(),
        };
        let folder_id = target.as_ref().and_then(|t| self.tab_folders.borrow().get(&t.id).cloned()).filter(|f| f != TEMPLATES_FOLDER);
        let folder = folder_id.as_ref().and_then(|id| self.folders.borrow().iter().find(|f| &f.id == id).map(|f| f.name.clone())).unwrap_or_default();
        let title = match (&target, usage) {
            (Some(t), TemplateUse::Insert) => t.title(),
            _ => tpl.name.clone(),
        };
        let user = {
            let real = glib::real_name().to_string_lossy().into_owned();
            if real.is_empty() || real == "Unknown" { glib::user_name().to_string_lossy().into_owned() } else { real }
        };
        let mut fields = crate::templates::Fields {
            now: None,
            title,
            folder,
            user,
            clipboard,
            selection,
            answers: Default::default(),
            german: crate::i18n::lang() == crate::i18n::Lang::De,
            date_format: crate::templates::locale_date_format(),
        };
        let questions = match crate::templates::questions(&tpl.text, &lib, &fields) {
            Ok(q) => q,
            Err(e) => {
                self.toast(&t!("Template “{}”: {}", tpl.name, e));
                return;
            }
        };
        if !questions.is_empty() {
            let dialog = adw::AlertDialog::new(Some(&tpl.name), None);
            let list = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).build();
            list.add_css_class("boxed-list");
            let mut entries = Vec::new();
            for q in &questions {
                let row = adw::EntryRow::builder().title(glib::markup_escape_text(&q.label).as_str()).text(&q.default).activates_default(true).build();
                list.append(&row);
                entries.push(row);
            }
            dialog.set_extra_child(Some(&list));
            dialog.add_responses(&[("cancel", t!("_Cancel")), ("ok", t!("_OK"))]);
            dialog.set_response_appearance("ok", adw::ResponseAppearance::Suggested);
            dialog.set_default_response(Some("ok"));
            dialog.set_close_response("cancel");
            if let Some(first) = entries.first() {
                first.connect_map(|e| {
                    e.grab_focus();
                });
            }
            if dialog.choose_future(&self.win).await != "ok" {
                return;
            }
            for (q, e) in questions.iter().zip(&entries) {
                fields.answers.insert(q.label.clone(), e.text().to_string());
            }
        }
        let out = match crate::templates::render(&tpl.text, &lib, &fields) {
            Ok(o) => o,
            Err(e) => {
                self.toast(&t!("Template “{}”: {}", tpl.name, e));
                return;
            }
        };
        let cursor_chars = out.cursor.map(|b| out.text[..b].chars().count() as i32);
        match usage {
            TemplateUse::Insert => {
                let Some(t) = target else { return };
                let buf = &t.buffer;
                buf.begin_user_action();
                if let Some((mut s, mut e)) = buf.selection_bounds() {
                    buf.delete(&mut s, &mut e);
                }
                let mut at = buf.iter_at_mark(&buf.get_insert());
                let start = at.offset();
                buf.insert(&mut at, &out.text);
                if let Some(c) = cursor_chars {
                    buf.place_cursor(&buf.iter_at_offset(start + c));
                }
                buf.end_user_action();
                t.view.grab_focus();
            }
            _ => {
                let t = self.new_note();
                *t.template.borrow_mut() = Some(crate::templates::Link::new(&tpl.id, &tpl.name, &fields, &out.text));
                t.set_text_irreversible(&out.text);
                if tpl.language.is_some() {
                    t.set_language_id(tpl.language.as_deref());
                }
                t.place_cursor(cursor_chars.map_or(CursorTarget::End, CursorTarget::Offset));
                if let Some(f) = folder_id.filter(|_| features::on(Feature::Folders)) {
                    self.tab_folders.borrow_mut().insert(t.id.clone(), f);
                }
                self.refresh_title(&t.id);
                self.refresh_tree();
                self.mark_dirty(&t.id);
                self.sync_ui();
            }
        }
    }

    /// Merges template changes into the open notes created from templates,
    /// after a pause (templates are usually edited in several steps).
    fn schedule_template_refresh(self: &Rc<Self>) {
        if let Some(id) = self.template_timer.borrow_mut().take() {
            id.remove();
        }
        let weak = self.weak();
        let id = glib::timeout_add_local_once(Duration::from_millis(1200), move || {
            if let Some(this) = weak.upgrade() {
                this.template_timer.borrow_mut().take();
                this.refresh_template_notes();
            }
        });
        *self.template_timer.borrow_mut() = Some(id);
    }

    /// Fills in each linked note's template again (with the note's own field
    /// values) and merges what changed in the template into the note, except
    /// where the note was edited.
    fn refresh_template_notes(self: &Rc<Self>) {
        let templates = self.template_list();
        let lib = TemplateLib(templates.clone());
        for t in self.text_tabs() {
            let Some(mut link) = t.template.borrow().clone() else { continue };
            if t.lock_state() != LockState::Open {
                continue;
            }
            let src = templates.iter().find(|x| x.id == link.id).or_else(|| templates.iter().find(|x| x.name.eq_ignore_ascii_case(&link.name)));
            let Some(src) = src else { continue };
            let Ok(out) = crate::templates::render(&src.text, &lib, &link.fields()) else { continue };
            if out.text == link.base {
                continue;
            }
            let text = t.text();
            let Some(merged) = crate::templates::merge(&link.base, &out.text, &text) else { continue };
            link.base = out.text;
            link.name = src.name.clone();
            *t.template.borrow_mut() = Some(link);
            if merged != text {
                t.replace_changed(&text, &merged);
                self.toast(&t!("“{}” updated from template “{}”", t.title(), src.name));
            }
            self.mark_dirty(&t.id);
        }
    }

    /// After an update: a read-only tab with the changes since the version
    /// that ran before.
    fn show_whats_new(self: &Rc<Self>, existing: bool) {
        let last = self.settings.borrow().last_version.clone();
        if last == crate::changelog::VERSION {
            return;
        }
        self.settings.borrow_mut().last_version = crate::changelog::VERSION.to_owned();
        self.schedule_persist();
        if last.is_empty() && !existing {
            return;
        }
        let Some(doc) = crate::changelog::whats_new(&last) else { return };
        let t = EditorTab::new("qn-whats-new".into(), self.shared.clone());
        self.add_tab(Tab::Text(t.clone()), true);
        t.show_document(t!("What's New"), &doc);
        self.refresh_title(&t.id);
        self.sync_ui();
    }

    /// Looks for a newer release on GitHub (in the background) and tells the
    /// user about it.
    fn check_for_updates(self: &Rc<Self>) {
        if !self.settings.borrow().update_check {
            return;
        }
        let this = self.clone();
        glib::spawn_future_local(async move {
            let Ok(Ok(Some((version, url)))) = gio::spawn_blocking(crate::changelog::newer_release).await else { return };
            let toast = adw::Toast::builder()
                .title(t!("QuickNotes {} is available", version))
                .button_label(t!("Download"))
                .timeout(0)
                .build();
            toast.connect_button_clicked(move |_| {
                gtk::UriLauncher::new(&url).launch(None::<&gtk::Window>, None::<&gio::Cancellable>, |_| {});
            });
            this.toasts.add_toast(toast);
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
        let notes = features::on(Feature::Folders);
        self.notes_btn.set_visible(notes);
        self.notes_tree.root.set_visible(notes && self.settings.borrow().notes_sidebar && !focus);
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

    /// Switches an optional feature on or off and updates menus, toolbars
    /// and the tabs whose type depends on it.
    fn set_feature(self: &Rc<Self>, feature: Feature, on: bool) {
        let all = {
            let mut s = self.settings.borrow_mut();
            s.features.set(feature, on);
            s.features
        };
        features::apply(all);
        self.tools_btn.set_visible(all.devtools);
        self.sync_layout();
        self.refresh_tree();
        // Context menus and the main menu are rebuilt for the new set.
        self.tab_styles.borrow_mut().clear();
        self.menu_context.set(None);
        for t in self.text_tabs() {
            t.features_changed();
        }
        self.sync_ui();
        self.schedule_persist();
    }

    fn show_preferences(self: &Rc<Self>) {
        let colors = TabStyle::COLORED
            .iter()
            .filter(|s| s.available())
            .map(|s| prefs::TabColor { key: s.key(), label: s.label(), default: s.default_color() })
            .collect();
        let mut settings = self.settings.borrow().clone();
        settings.theme = self.shared.theme.get().id.clone();
        let app_password = self.vault.borrow().is_some();
        prefs::show(&self.win, &settings, app_password, colors, Rc::new(PrefsBridge(self.weak())));
    }

    // ===== data import ======================================================

    /// Imports a CSV/TSV file or a spreadsheet as Markdown table.
    fn import_data(self: &Rc<Self>, t: Rc<EditorTab>) {
        let filter = gtk::FileFilter::new();
        filter.set_name(Some(t!("Tables (CSV, TSV, Excel, ODS)")));
        for ext in ["csv", "tsv", "tab", "txt", "xlsx", "xlsm", "xlsb", "xls", "ods"] {
            filter.add_suffix(ext);
        }
        let all = gtk::FileFilter::new();
        all.set_name(Some(t!("All Files")));
        all.add_pattern("*");
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        filters.append(&all);
        let dialog = gtk::FileDialog::builder().title(t!("Import Data")).modal(true).filters(&filters).default_filter(&filter).build();
        if let Some(dir) = t.base_dir() {
            dialog.set_initial_folder(Some(&gio::File::for_path(dir)));
        }
        let this = self.clone();
        glib::spawn_future_local(async move {
            let Ok(file) = dialog.open_future(Some(&this.win)).await else { return };
            let Some(path) = file.path() else { return };
            let source = if crate::import::is_spreadsheet(&path) {
                match crate::import::read_spreadsheet(&path) {
                    Ok(sheets) => ImportSource::Sheets(sheets),
                    Err(e) => return this.toast(&t!("Could not read {}: {}", path.display(), e)),
                }
            } else {
                match std::fs::read(&path) {
                    Ok(bytes) => ImportSource::Text(crate::import::decode(&bytes)),
                    Err(e) => return this.toast(&t!("Could not read {}: {}", path.display(), e)),
                }
            };
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            this.import_dialog(t, &name, source).await;
        });
    }

    /// Inserts the clipboard's tab or comma separated text as table.
    fn paste_table(self: &Rc<Self>, t: Rc<EditorTab>) {
        let this = self.clone();
        let clipboard = self.win.clipboard();
        glib::spawn_future_local(async move {
            let text = clipboard.read_text_future().await.ok().flatten().map(|s| s.to_string()).unwrap_or_default();
            let rows = crate::import::read_delimited(&text, None).unwrap_or_default();
            if rows.len() < 2 || rows.iter().all(|r| r.len() < 2) {
                return this.toast(t!("The clipboard holds no table data"));
            }
            this.insert_rows(&t, &rows, true);
        });
    }

    fn insert_rows(&self, t: &EditorTab, rows: &[Vec<String>], header: bool) {
        let (table, cut) = crate::import::to_markdown(rows, header, "");
        t.insert_block(&table, 0);
        if cut {
            self.toast(&t!("Only the first {} rows were imported", crate::import::MAX_ROWS));
        }
    }

    /// Lets the user pick sheet, separator and header row, then inserts the table.
    async fn import_dialog(self: &Rc<Self>, t: Rc<EditorTab>, name: &str, source: ImportSource) {
        let dialog = adw::AlertDialog::new(Some(t!("Import Data")), Some(name));
        let list = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).build();
        list.add_css_class("boxed-list");
        let sheet = match &source {
            ImportSource::Sheets(sheets) if sheets.len() > 1 => {
                let names: Vec<&str> = sheets.iter().map(|s| s.name.as_str()).collect();
                let row = adw::ComboRow::builder().title(t!("Sheet")).model(&gtk::StringList::new(&names)).build();
                // The first sheet with data.
                if let Some(i) = sheets.iter().position(|s| !s.rows.is_empty()) {
                    row.set_selected(i as u32);
                }
                list.append(&row);
                Some(row)
            }
            _ => None,
        };
        let delimiter = match &source {
            ImportSource::Text(text) => {
                let mut labels = vec![t!("Automatic")];
                labels.extend(crate::import::DELIMITERS.iter().map(|(_, l)| crate::i18n::tr(l)));
                let row = adw::ComboRow::builder().title(t!("Separator")).model(&gtk::StringList::new(&labels)).build();
                let detected = crate::import::detect_delimiter(text);
                let auto = crate::import::DELIMITERS.iter().find(|(d, _)| *d == detected).map_or("", |(_, l)| crate::i18n::tr(l));
                row.set_subtitle(&t!("Detected: {}", auto));
                list.append(&row);
                Some(row)
            }
            _ => None,
        };
        let header = adw::SwitchRow::builder().title(t!("First row is header")).active(true).build();
        list.append(&header);
        let info = gtk::Label::builder().xalign(0.0).wrap(true).margin_top(8).build();
        info.add_css_class("caption");
        info.add_css_class("dim-label");
        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.append(&list);
        content.append(&info);
        dialog.set_extra_child(Some(&content));
        dialog.add_response("cancel", t!("_Cancel"));
        dialog.add_response("import", t!("_Import"));
        dialog.set_response_appearance("import", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("import"));
        dialog.set_close_response("cancel");

        let source = Rc::new(source);
        let rows = {
            let (source, sheet, delimiter) = (source.clone(), sheet.clone(), delimiter.clone());
            move || -> Vec<Vec<String>> {
                match &*source {
                    ImportSource::Sheets(sheets) => {
                        let i = sheet.as_ref().map_or(0, |s| s.selected() as usize);
                        sheets.get(i).map(|s| s.rows.clone()).unwrap_or_default()
                    }
                    ImportSource::Text(text) => {
                        let d = delimiter
                            .as_ref()
                            .map(|r| r.selected() as usize)
                            .filter(|&i| i > 0)
                            .and_then(|i| crate::import::DELIMITERS.get(i - 1))
                            .map(|(d, _)| *d);
                        crate::import::read_delimited(text, d).unwrap_or_default()
                    }
                }
            }
        };
        let rows = Rc::new(rows);
        let update = {
            let (rows, info, dialog, header) = (rows.clone(), info.clone(), dialog.clone(), header.clone());
            move || {
                let rows = rows();
                let width = rows.iter().map(Vec::len).max().unwrap_or(0);
                let body = rows.len().saturating_sub(usize::from(header.is_active()));
                info.set_text(&t!("{} rows, {} columns", body, width));
                dialog.set_response_enabled("import", !rows.is_empty());
            }
        };
        let update = Rc::new(update);
        update();
        if let Some(s) = &sheet {
            let u = update.clone();
            s.connect_selected_notify(move |_| u());
        }
        if let Some(d) = &delimiter {
            let u = update.clone();
            d.connect_selected_notify(move |_| u());
        }
        let u = update.clone();
        header.connect_active_notify(move |_| u());

        if dialog.choose_future(&self.win).await == "import" {
            self.insert_rows(&t, &rows(), header.is_active());
        }
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
        self.refresh_tree();

        self.guard.set(true);
        self.update_lock_actions();
        match self.current() {
            Some(Tab::Text(t)) => {
                let lang = t.language_id();
                let idx = self.lang_ids.iter().position(|l| *l == lang).unwrap_or(0);
                self.lang_drop.set_selected(idx as u32);
                self.lang_drop.set_visible(true);
                self.set_toggle("preview", t.preview_kind().is_some() && !t.transient.get(), t.preview_toggle_state());
                self.set_toggle("wrap", true, t.wrap());
                let exportable = t.preview_kind().is_some_and(PreviewKind::is_document);
                let mindmap = t.preview_kind() == Some(PreviewKind::Mindmap);
                self.set_enabled("export-html", exportable);
                self.set_enabled("export-pdf", exportable || mindmap);
                self.set_enabled("export-svg", mindmap);
                let slides = t.preview_kind() == Some(PreviewKind::Slides);
                self.set_presentable(slides);
                self.set_calc_tools(t.is_calc());
                let markdown = matches!(t.preview_kind(), Some(PreviewKind::Markdown | PreviewKind::Slides | PreviewKind::Mindmap | PreviewKind::Adventure));
                self.set_format_enabled(markdown);
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
                self.set_format_enabled(false);
                self.set_presentable(false);
                self.set_calc_tools(false);
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
            app_password: self.vault.borrow().is_some(),
            features: features::current(),
            lock: match &current {
                Some(Tab::Text(t)) if t.path.borrow().is_none() || t.lock().is_some() => Some(t.lock_state()),
                _ => None,
            },
            data: matches!(&current, Some(Tab::Text(t)) if t.data_kind().is_some()),
            linked: matches!(&current, Some(Tab::Text(t)) if t.template.borrow().is_some()),
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
            let key = (TabStyle::of(&entry.tab), t.emmet_syntax().is_some(), t.data_kind().is_some());
            if styles.get(id) != Some(&key) {
                t.set_context_menus(editor_context_menu(key.0, key.1, key.2).upcast(), Some(table_menu().upcast()));
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
        let iter = t.buffer.iter_at_mark(&t.buffer.get_insert());
        let rect = t.view.iter_location(&iter);
        let (x, y) = t.view.buffer_to_window_coords(gtk::TextWindowType::Widget, rect.x(), rect.y());
        t.show_popover(&popover, gdk::Rectangle::new(x, y, 1, rect.height()));
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
        let iter = match t.buffer.selection_bounds() {
            Some((start, _)) => start,
            None => t.buffer.iter_at_mark(&t.buffer.get_insert()),
        };
        let rect = t.view.iter_location(&iter);
        let (x, y) = t.view.buffer_to_window_coords(gtk::TextWindowType::Widget, rect.x(), rect.y());

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
        t.show_popover(&popover, gdk::Rectangle::new(x, y, 1, rect.height()));
        entry.grab_focus();
    }

    fn set_calc_tools(&self, on: bool) {
        self.set_enabled("unit-converter", on);
        self.convert_btn.set_visible(on);
    }

    /// Opens the unit converter; its line goes below the cursor line.
    fn unit_converter(self: &Rc<Self>) {
        let Some(t) = self.current_text().filter(|t| t.is_calc()) else { return };
        let weak = Rc::downgrade(&t);
        crate::convert_dialog::open(&self.win, move |line| {
            let Some(t) = weak.upgrade() else { return };
            t.insert_line(line);
            t.view.grab_focus();
        });
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
        if t.is_kanban() && !selected {
            let (cards, done) = kanban::stats(&kanban::parse(&text));
            self.status_counts.set_text(&t!("{} cards  ·  {} done", cards, done));
            return;
        }
        let c = stats::count(&text);
        let mut label = t!("{} words  ·  {} characters  ·  {} paragraphs", c.words, c.chars, c.paragraphs);
        if selected {
            label = t!("Selection: {}", label);
        }
        if t.checklists_enabled() {
            let (done, total) = checklist::progress(&text);
            if total > 0 {
                label = format!("{label}  ·  {}", t!("✓ {} of {} done", done, total));
            }
        }
        self.status_counts.set_text(&label);
    }

    /// Errors and warnings of the current JSON, YAML or XML tab in the status bar.
    fn update_problems_status(&self) {
        let Some(t) = self.current_text().filter(|t| t.data_kind().is_some()) else {
            self.status_problems.set_visible(false);
            return;
        };
        let (errors, warnings) = t.problem_counts();
        self.problem_errors.set_text(&errors.to_string());
        self.problem_warnings.set_text(&warnings.to_string());
        let mut tip = vec![t!("Problems list (Ctrl+Shift+M)").to_owned()];
        if let Some(schema) = t.schema_label() {
            tip.push(schema);
        }
        self.status_problems.set_tooltip_text(Some(&tip.join("\n")));
        self.status_problems.set_visible(true);
    }

    fn update_status(&self) {
        self.schedule_counts();
        self.update_problems_status();
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
    Kanban,
    Cards,
    Mindmap,
    Adventure,
    Image,
}

impl TabStyle {
    const ALL: [Self; 10] =
        [Self::Note, Self::Code, Self::Document, Self::Slides, Self::Calc, Self::Kanban, Self::Cards, Self::Mindmap, Self::Adventure, Self::Image];

    fn of(tab: &Tab) -> Self {
        let Tab::Text(t) = tab else { return Self::Image };
        match t.preview_kind() {
            Some(PreviewKind::Slides) => Self::Slides,
            Some(PreviewKind::Kanban) => Self::Kanban,
            Some(PreviewKind::Flashcards) => Self::Cards,
            Some(PreviewKind::Mindmap) => Self::Mindmap,
            Some(PreviewKind::Adventure) => Self::Adventure,
            Some(kind) if kind.is_document() => Self::Document,
            _ if t.is_calc() => Self::Calc,
            _ if t.language_id().is_none() => Self::Note,
            _ => Self::Code,
        }
    }

    /// The type of a closed tab of the notes library.
    fn of_meta(meta: &TabMeta) -> Self {
        if meta.kind == TabKind::Image {
            return Self::Image;
        }
        match meta.language.as_deref() {
            None => Self::Note,
            Some(slides::LANGUAGE) if features::on(Feature::Presentations) => Self::Slides,
            Some(kanban::LANGUAGE) if features::on(Feature::Kanban) => Self::Kanban,
            Some(crate::flashcards::LANGUAGE) if features::on(Feature::Flashcards) => Self::Cards,
            Some(crate::mindmap::LANGUAGE) if features::on(Feature::Mindmaps) => Self::Mindmap,
            Some(crate::adventure::LANGUAGE) if features::on(Feature::GmTools) => Self::Adventure,
            Some("calc") => Self::Calc,
            Some(
                "markdown" | "rst" | "html" | "plantuml" | slides::LANGUAGE | kanban::LANGUAGE | crate::flashcards::LANGUAGE
                | crate::mindmap::LANGUAGE | crate::adventure::LANGUAGE,
            ) => {
                Self::Document
            }
            Some(_) => Self::Code,
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Note => "text-x-generic-symbolic",
            Self::Code => "text-x-script-symbolic",
            Self::Document => "x-office-document-symbolic",
            Self::Slides => "x-office-presentation-symbolic",
            Self::Calc => "accessories-calculator-symbolic",
            Self::Kanban => "view-grid-symbolic",
            Self::Cards => "starred-symbolic",
            Self::Mindmap => "qn-format-mindmap-symbolic",
            Self::Adventure => "qn-format-dice-symbolic",
            Self::Image => "image-x-generic-symbolic",
        }
    }

    /// Types with their own colour (plain notes keep the default look).
    const COLORED: [Self; 9] =
        [Self::Code, Self::Document, Self::Slides, Self::Calc, Self::Kanban, Self::Cards, Self::Mindmap, Self::Adventure, Self::Image];

    /// Whether tabs of this type exist with the current features.
    fn available(&self) -> bool {
        match self {
            Self::Slides => features::on(Feature::Presentations),
            Self::Kanban => features::on(Feature::Kanban),
            Self::Cards => features::on(Feature::Flashcards),
            Self::Mindmap => features::on(Feature::Mindmaps),
            Self::Adventure => features::on(Feature::GmTools),
            _ => true,
        }
    }

    /// Key in `Settings::tab_colors`.
    fn key(self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Code => "code",
            Self::Document => "document",
            Self::Slides => "slides",
            Self::Calc => "calc",
            Self::Kanban => "kanban",
            Self::Cards => "flashcards",
            Self::Mindmap => "mindmap",
            Self::Adventure => "adventure",
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
            Self::Kanban => t!("Kanban boards"),
            Self::Cards => t!("Flash cards"),
            Self::Mindmap => t!("Mind maps"),
            Self::Adventure => t!("Adventures"),
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
            Self::Kanban => "#e5a50a",
            Self::Cards => "#c061cb",
            Self::Mindmap => "#e01b24",
            Self::Adventure => "#986a44",
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
            Self::Kanban => "tab-kanban",
            Self::Cards => "tab-flashcards",
            Self::Mindmap => "tab-mindmap",
            Self::Adventure => "tab-adventure",
            Self::Image => "tab-image",
        }
    }

    fn is_text(self) -> bool {
        self != Self::Image
    }

    fn exportable(self) -> bool {
        matches!(self, Self::Document | Self::Slides | Self::Mindmap)
    }
}

fn b64() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

/// The title of a closed tab of the notes library.
/// Id of the library folder holding the templates.
pub const TEMPLATES_FOLDER: &str = "qn-templates";

/// What a picked template is used for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TemplateUse {
    New,
    Insert,
    Edit,
}

#[derive(Clone, Debug)]
struct TemplateNote {
    id: String,
    name: String,
    language: Option<String>,
    text: String,
}

struct TemplateLib(Vec<TemplateNote>);

impl crate::templates::Lookup for TemplateLib {
    fn template(&self, name: &str) -> Option<String> {
        let name = name.trim().to_lowercase();
        self.0.iter().find(|t| t.name.trim().to_lowercase() == name).map(|t| t.text.clone())
    }
}

fn library_title(meta: &TabMeta) -> String {
    meta.label
        .clone()
        .or_else(|| meta.title.clone())
        .or_else(|| meta.path.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| t!("Untitled note").to_owned())
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
    fn github_sign_in(&self, token: bool, done: Box<dyn Fn(Result<String, String>)>) {
        self.with(|w| w.github_sign_in(token, done));
    }
    fn github_sign_out(&self) {
        self.with(|w| w.github_sign_out());
    }
    fn github_connect(&self, repo: String, done: Box<dyn Fn(Result<String, String>)>) {
        self.with(|w| w.github_connect(repo, done));
    }
    fn github_disconnect(&self) {
        self.with(|w| w.github_disconnect());
    }
    fn set_github_sync(&self, on: bool) {
        self.with(|w| {
            w.settings.borrow_mut().github_sync = on;
            w.persist_all();
        });
    }
    fn github_sync_now(&self) {
        self.with(|w| w.github_sync_now());
    }
    fn set_sync_kind(&self, kind: &str) {
        self.with(|w| {
            w.settings.borrow_mut().sync_kind = kind.to_owned();
            w.schedule_persist();
        });
    }
    fn sync_connect(&self, target: crate::remote::Target, secret: String, done: Box<dyn Fn(Result<String, String>)>) {
        self.with(|w| w.sync_connect(target, secret, done));
    }
    fn nextcloud_sign_in(&self, server: String, folder: String, done: Box<dyn Fn(Result<String, String>)>) {
        self.with(|w| w.nextcloud_sign_in(server, folder, done));
    }
    fn sync_disconnect(&self) {
        self.with(|w| w.sync_disconnect());
    }
    fn sync_description(&self) -> Option<String> {
        let mut out = None;
        self.with(|w| out = w.settings.borrow().sync_target.as_ref().map(|t| format!("{}: {}", t.name(), t.describe())));
        out
    }
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

    fn set_feature(&self, feature: Feature, on: bool) {
        self.with(|w| w.set_feature(feature, on));
    }

    fn set_emmet_tab(&self, on: bool) {
        self.with(|w| {
            w.settings.borrow_mut().emmet_tab = on;
            w.shared.emmet_tab.set(on);
            w.schedule_persist();
        });
    }

    fn set_update_check(&self, on: bool) {
        self.with(|w| {
            w.settings.borrow_mut().update_check = on;
            w.schedule_persist();
        });
    }

    fn set_yaml_rule(&self, key: &str, on: bool) {
        self.with(|w| {
            let rules = {
                let mut s = w.settings.borrow_mut();
                s.yaml_rules.set(key, on);
                s.yaml_rules
            };
            w.shared.yaml_rules.set(rules);
            for t in w.text_tabs() {
                if t.data_kind() == Some(crate::data_assist::Kind::Yaml) {
                    t.lint();
                }
            }
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

    fn app_password(&self, action: prefs::PasswordAction, done: Box<dyn Fn(bool)>) {
        self.with(|w| match action {
            prefs::PasswordAction::Set => w.set_app_password(done),
            prefs::PasswordAction::Change => w.change_app_password(done),
            prefs::PasswordAction::Remove => w.remove_app_password(done),
        });
    }

    fn lock_app(&self) {
        self.with(|w| w.lock_app());
    }

    fn open_data_dir(&self) {
        self.with(|w| {
            let launcher = gtk::FileLauncher::new(Some(&gio::File::for_path(store::data_dir())));
            launcher.launch(Some(&w.win), None::<&gio::Cancellable>, |_| {});
        });
    }
}

/// Minimum length of new passwords.
const MIN_PASSWORD: usize = 8;

/// A password dialog: texts and whether the password is entered twice.
struct PasswordPrompt<'a> {
    heading: &'a str,
    body: &'a str,
    confirm: bool,
    ok: &'a str,
    /// Extra response to go on without a password.
    skip: Option<&'a str>,
    /// Shown below the field (e.g. after a wrong password).
    error: Option<&'a str>,
}

enum PasswordAnswer {
    Cancel,
    Skip,
    Password(Zeroizing<String>),
}

/// What the main menu depends on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct MenuContext {
    style: Option<TabStyle>,
    /// Optional features that are on.
    features: Features,
    has_file: bool,
    /// Emmet is available for the current tab's language.
    emmet: bool,
    /// The app password is on (offers *Lock QuickNotes*).
    app_password: bool,
    /// Password state of the current tab, if it can have one.
    lock: Option<LockState>,
    /// The current tab is JSON, YAML or XML (with the feature on).
    data: bool,
    /// The current tab was created from a template.
    linked: bool,
}

/// Tab password commands: all of them (tab context menu, enabled per tab) or
/// those that apply to `state`.
fn lock_section(all: bool) -> gio::Menu {
    lock_items(all, None)
}

fn lock_items(all: bool, state: Option<LockState>) -> gio::Menu {
    let protect = (t!("Protect with Password…"), "win.tab-protect");
    let unlocked = [
        (t!("Lock Tab"), "win.tab-lock"),
        (t!("Change Tab Password…"), "win.tab-change-password"),
        (t!("Remove Tab Password…"), "win.tab-remove-password"),
    ];
    let mut items = Vec::new();
    if all || state == Some(LockState::Open) {
        items.push(protect);
    }
    if all || state == Some(LockState::Unlocked) {
        items.extend(unlocked);
    }
    menu_section(&items)
}

/// Popover for a menu button whose submenus open beside the menu.
fn nested_menu(model: &impl IsA<gio::MenuModel>) -> gtk::PopoverMenu {
    gtk::PopoverMenu::from_model_full(model, gtk::PopoverMenuFlags::NESTED)
}

fn menu_section(items: &[(&str, &str)]) -> gio::Menu {
    let s = gio::Menu::new();
    for (label, action) in items {
        s.append(Some(label), Some(action));
    }
    s
}

fn export_section(style: TabStyle) -> gio::Menu {
    if style == TabStyle::Mindmap {
        return menu_section(&[(t!("Export as PDF…"), "win.export-pdf"), (t!("Export as SVG…"), "win.export-svg")]);
    }
    let mut items = vec![
        (t!("Export as HTML…"), "win.export-html"),
        (t!("Export as PDF…"), "win.export-pdf"),
        (t!("Export as Image…"), "win.export-image"),
    ];
    if style != TabStyle::Slides {
        items.push((t!("Export as EPUB…"), "win.export-epub"));
    }
    menu_section(&items)
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

    let mut file: Vec<(&str, &str)> = vec![(t!("New Note"), "win.new-note"), (t!("New Calculation"), "win.new-calculation")];
    if context.features.presentations {
        file.push((t!("New Presentation"), "win.new-presentation"));
    }
    if context.features.kanban {
        file.push((t!("New Kanban Board"), "win.new-kanban"));
    }
    if context.features.flashcards {
        file.push((t!("New Flash Cards"), "win.new-flashcards"));
    }
    if context.features.mindmaps {
        file.push((t!("New Mind Map"), "win.new-mindmap"));
    }
    if context.features.gmtools {
        file.push((t!("New Adventure"), "win.new-adventure"));
    }
    file.push((t!("New from Template…"), "win.new-from-template"));
    file.push((t!("Open File…"), "win.open"));
    if context.features.folders {
        file.push((t!("Import Obsidian Vault…"), "win.import-obsidian"));
    }
    if text {
        file.extend([(t!("Save"), "win.save"), (t!("Save As…"), "win.save-as")]);
    }
    if context.has_file {
        file.push((t!("Reload from Disk"), "win.reload"));
    }
    menu.append_section(None, &menu_section(&file));
    if context.features.sync {
        menu.append_section(None, &menu_section(&[(t!("Sync Now"), "win.github-sync")]));
    }
    if let Some(state) = context.lock.filter(|s| *s != LockState::Locked) {
        menu.append_section(None, &lock_items(false, Some(state)));
    }
    if context.app_password {
        menu.append_section(None, &menu_section(&[(t!("Lock QuickNotes"), "win.lock-app")]));
    }

    if let Some(style) = context.style {
        if style.exportable() {
            menu.append_section(None, &export_section(style));
        }
        if style == TabStyle::Slides {
            menu.append_section(None, &presentation_section());
        }
        if style == TabStyle::Adventure {
            menu.append_section(None, &character_section());
        }
    }

    let mut edit: Vec<(&str, &str)> = Vec::new();
    if text {
        edit.extend([(t!("Find…"), "win.find"), (t!("Find and Replace…"), "win.replace")]);
    }
    if context.style.is_some() {
        edit.push((t!("Rename Tab…"), "win.rename"));
    }
    if text {
        edit.extend([(t!("Insert Template…"), "win.insert-template"), (t!("Save as Template…"), "win.save-as-template")]);
    }
    if context.linked {
        edit.push((t!("Detach from Template"), "win.detach-template"));
    }
    edit.push((t!("Edit Templates…"), "win.edit-templates"));
    edit.push((t!("Reopen Closed Tab"), "win.reopen-closed"));
    menu.append_section(None, &menu_section(&edit));
    if context.emmet {
        menu.append_section(None, &emmet_section());
    }
    if context.data {
        menu.append_section(None, &data_section());
    }

    let mut view: Vec<(&str, &str)> = Vec::new();
    if text {
        view.push((t!("Word Wrap"), "win.wrap"));
    }
    if matches!(context.style, Some(TabStyle::Document | TabStyle::Slides | TabStyle::Cards)) {
        view.push((t!("Formatting Toolbar"), "win.format-bar"));
    }
    view.push((t!("Distraction-Free Mode"), "win.focus-mode"));
    if context.features.flashcards {
        view.push((t!("Learning Overview"), "win.cards-overview"));
    }
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

/// *Insert Character* with the sheet presets of the GM tools.
fn character_section() -> gio::Menu {
    let sub = gio::Menu::new();
    for (id, label, _) in crate::adventure::presets() {
        sub.append(Some(crate::i18n::tr(label)), Some(&format!("win.insert-character::{id}")));
    }
    let section = gio::Menu::new();
    section.append_submenu(Some(t!("Insert Character")), &sub);
    section
}

/// Schema commands for JSON, YAML and XML documents.
fn data_section() -> gio::Menu {
    menu_section(&[(t!("Assign Schema…"), "win.assign-schema"), (t!("Problems List"), "win.problems")])
}

fn emmet_section() -> gio::Menu {
    menu_section(&[
        (t!("Expand Emmet Abbreviation"), "win.emmet-expand"),
        (t!("Wrap with Emmet Abbreviation…"), "win.emmet-wrap"),
    ])
}

/// Extra entries for an editor's right-click menu.
fn editor_context_menu(style: TabStyle, emmet: bool, data: bool) -> gio::Menu {
    let menu = gio::Menu::new();
    if emmet {
        menu.append_section(None, &emmet_section());
    }
    if data {
        menu.append_section(None, &data_section());
    }
    if style.is_text() {
        menu.append_section(None, &menu_section(&[(t!("Insert Template…"), "win.insert-template")]));
    }
    if matches!(style, TabStyle::Note | TabStyle::Document | TabStyle::Slides | TabStyle::Cards) {
        menu.append_section(
            None,
            &menu_section(&[
                (t!("Insert Table"), "win.table-insert"),
                (t!("Import Data (CSV, Excel, ODS)…"), "win.table-import"),
                (t!("Paste as Table"), "win.table-paste"),
            ]),
        );
    }
    if features::on(Feature::DevTools) {
        let dev = gio::Menu::new();
        dev.append_section(None, &quick_tools());
        dev.append_section(None, &tool_submenus());
        let section = gio::Menu::new();
        section.append_submenu(Some(t!("Developer Tools")), &dev);
        menu.append_section(None, &section);
    }
    if style.exportable() {
        menu.append_section(None, &export_section(style));
    }
    if style == TabStyle::Slides {
        menu.append_section(None, &presentation_section());
    }
    if style == TabStyle::Calc {
        menu.append_section(None, &menu_section(&[(t!("Unit Converter…"), "win.unit-converter")]));
    }
    if style == TabStyle::Adventure {
        menu.append_section(None, &character_section());
    }
    if features::on(Feature::Geocaching) {
        menu.append_section(None, &geocaching_menu());
    }
    menu
}

/// Where an image export goes.
enum ImageTarget {
    File(PathBuf),
    /// A folder and the name stem for numbered files.
    Folder(PathBuf, String),
}

impl ImageTarget {
    /// Writes the images; returns the file or folder.
    fn write(&self, images: &[Vec<u8>], ext: &str) -> Result<PathBuf, String> {
        match self {
            ImageTarget::File(path) => {
                std::fs::write(path, images.first().ok_or("no image")?).map_err(|e| e.to_string())?;
                Ok(path.clone())
            }
            ImageTarget::Folder(dir, stem) => {
                let digits = images.len().to_string().len().max(2);
                for (i, image) in images.iter().enumerate() {
                    let file = dir.join(format!("{stem}-{:0digits$}.{ext}", i + 1));
                    std::fs::write(&file, image).map_err(|e| e.to_string())?;
                }
                Ok(dir.clone())
            }
        }
    }
}

/// Data to import as table.
enum ImportSource {
    /// Delimited text (CSV, TSV, …).
    Text(String),
    Sheets(Vec<crate::import::Sheet>),
}

/// Markdown formatting commands: action name, accelerators and operation
/// (text colours are added from [`md_format::COLORS`]).
const FORMAT_ACTIONS: &[(&str, &[&str], md_format::Op)] = &[
    ("md-bold", &["<Control>b"], md_format::Op::Bold),
    ("md-italic", &["<Control>i"], md_format::Op::Italic),
    ("md-strike", &[], md_format::Op::Strike),
    ("md-code", &[], md_format::Op::Code),
    ("md-h1", &[], md_format::Op::Heading(1)),
    ("md-h2", &[], md_format::Op::Heading(2)),
    ("md-h3", &[], md_format::Op::Heading(3)),
    ("md-h4", &[], md_format::Op::Heading(4)),
    ("md-h5", &[], md_format::Op::Heading(5)),
    ("md-h6", &[], md_format::Op::Heading(6)),
    ("md-paragraph", &[], md_format::Op::Heading(0)),
    ("md-quote", &[], md_format::Op::Quote),
    ("md-bullet", &[], md_format::Op::Bullet),
    ("md-numbered", &[], md_format::Op::Numbered),
    ("md-task", &[], md_format::Op::Task),
    ("md-code-block", &[], md_format::Op::CodeBlock),
    ("md-math", &[], md_format::Op::Math),
    ("md-math-block", &[], md_format::Op::MathBlock),
    ("md-music", &[], md_format::Op::Music),
    ("md-link", &["<Control>k"], md_format::Op::Link),
    ("md-image", &[], md_format::Op::Image),
    ("md-rule", &[], md_format::Op::Rule),
    ("md-color-none", &[], md_format::Op::Color(None)),
];

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

/// Table commands for the editor's context menu (cursor in a table).
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

/// The sync target in use.
enum ActiveSync {
    GitHub(String),
    Remote(crate::remote::Target),
}

impl ActiveSync {
    /// Identifies it (sync state).
    fn id(&self) -> String {
        match self {
            ActiveSync::GitHub(repo) => repo.clone(),
            ActiveSync::Remote(t) => t.id(),
        }
    }

    fn name(&self) -> &'static str {
        match self {
            ActiveSync::GitHub(_) => "GitHub",
            ActiveSync::Remote(t) => t.name(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ExportFormat {
    Html,
    Pdf,
    /// Mind maps only.
    Svg,
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
