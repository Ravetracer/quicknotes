//! A text tab: GtkSourceView editor plus an optional live preview
//! (Markdown, reStructuredText, PlantUML, HTML, presentations) in a split pane.

use crate::store::{TabKind, TabMeta};
use crate::calc::{self, LineResult};
use crate::themes::Theme;
use crate::{checklist, crypto, emmet, format_bar, html, kanban, langs, markdown, md_format, plantuml, rst, slides, table};
use gtk::prelude::*;
use gtk::{gdk, gio, glib};
use sourceview5::prelude::*;
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;
use webkit6::prelude::*;
use zeroize::Zeroizing;

/// State shared by all tabs of the window.
pub struct Shared {
    pub theme: Cell<&'static Theme>,
    /// Tab expands Emmet abbreviations (in supported languages).
    pub emmet_tab: Cell<bool>,
    /// Preview zoom factor, follows the editor font size.
    pub zoom: Cell<f64>,
    pub plantuml_command: RefCell<Option<String>>,
    /// Formatting toolbar above Markdown editors.
    pub format_bar: Cell<bool>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PreviewKind {
    Markdown,
    PlantUml,
    Rst,
    Html,
    Slides,
    /// Kanban board: the board replaces the editor (the source can be shown next to it).
    Kanban,
}

impl PreviewKind {
    fn for_language(id: Option<&str>) -> Option<Self> {
        match id {
            Some("markdown") => Some(Self::Markdown),
            Some("plantuml") => Some(Self::PlantUml),
            Some("rst") => Some(Self::Rst),
            Some("html") => Some(Self::Html),
            Some(slides::LANGUAGE) => Some(Self::Slides),
            Some(kanban::LANGUAGE) => Some(Self::Kanban),
            _ => None,
        }
    }

    /// Documents that can be exported to HTML/PDF.
    pub fn is_document(self) -> bool {
        matches!(self, Self::Markdown | Self::Rst | Self::Slides)
    }

    /// Shown automatically when such a file is opened (HTML files are often
    /// edited as code, so their preview is opt-in).
    pub fn auto_preview(self) -> bool {
        !matches!(self, Self::Html | Self::Kanban)
    }

    fn render_delay_ms(self) -> u64 {
        match self {
            Self::PlantUml => 700,
            Self::Html => 400,
            Self::Markdown | Self::Rst | Self::Slides => 250,
            Self::Kanban => 60,
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

/// Page loaded in the preview.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shell {
    /// A user HTML document.
    None,
    /// The document shell (Markdown, reStructuredText, PlantUML).
    Doc,
    /// The presentation preview.
    Slides,
    /// The Kanban board.
    Kanban,
}

struct Preview {
    webview: webkit6::WebView,
    /// The current page finished loading (navigations are blocked from then on).
    loaded: Cell<bool>,
    shell: Cell<Shell>,
    /// JavaScript to run once the current page has loaded.
    pending: RefCell<Option<String>>,
}

pub type WidgetHook = Box<dyn Fn(&gtk::Widget)>;
pub type MessageHook = Box<dyn Fn(&str)>;

/// Result column of a calculation sheet (right gutter).
struct CalcColumn {
    renderer: sourceview5::GutterRendererText,
    results: Rc<RefCell<Vec<LineResult>>>,
}

const CALC_LANGUAGE: &str = "calc";
const TASK_DONE_TAG: &str = "qn-task-done";
const TASK_BOX_TAG: &str = "qn-task-box";
const CALC_PAD: i32 = 14;
const TOP_MARGIN: i32 = 6;
const FOCUS_TOP_MARGIN: i32 = 48;
const LEFT_MARGIN: i32 = 6;
const RIGHT_MARGIN: i32 = 12;
/// Width of the text column in distraction-free mode, in characters.
const FOCUS_COLUMNS: i32 = 90;

/// Associated data of password-protected tab contents.
const TAB_AAD: &[u8] = b"quicknotes-locked-tab";

/// Password protection of a tab.
pub struct TabLock {
    pub lock: crypto::Lock,
    /// The tab's key while unlocked.
    key: Option<crypto::Key>,
    /// The sealed content while locked.
    sealed: Vec<u8>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LockState {
    /// No tab password.
    Open,
    /// Tab password set, content shown.
    Unlocked,
    /// Tab password set, content hidden.
    Locked,
}

pub struct EditorTab {
    pub id: String,
    /// Switches between the editor (`root`) and the password prompt of a locked tab.
    pub frame: gtk::Stack,
    pub root: gtk::Paned,
    format_bar: gtk::Box,
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
    /// Distraction-free mode: no preview, no line numbers, centred text.
    focus: Cell<bool>,
    /// Pane size the split ratio was last applied for; -1 forces re-applying.
    split_max: Cell<i32>,
    /// Editor share of the editor/preview split (0..1), kept across sessions.
    pub split: Cell<Option<f64>>,
    preview_timer: RefCell<Option<glib::SourceId>>,
    rendering: Cell<bool>,
    rerender: Cell<bool>,
    search: RefCell<Option<sourceview5::SearchContext>>,
    /// Called with the preview widget when it is created (used to add drop targets).
    pub webview_hook: RefCell<Option<WidgetHook>>,
    /// Shows a short message to the user (e.g. "Copied …").
    pub message_hook: RefCell<Option<MessageHook>>,
    calc: RefCell<Option<CalcColumn>>,
    calc_timer: RefCell<Option<glib::SourceId>>,
    /// Presentations: source lines of the slides (as of the last render) and
    /// the slide shown in the preview.
    slide_lines: RefCell<Vec<(usize, usize)>>,
    shown_slide: Cell<usize>,
    /// Last inserted Emmet snippet; while it is active, Tab moves between its fields.
    snippet: RefCell<Option<sourceview5::Snippet>>,
    tab_lock: RefCell<Option<TabLock>>,
    lock_entry: gtk::PasswordEntry,
    lock_error: gtk::Label,
    /// Called when the tab was locked, unlocked or its protection changed.
    pub lock_hook: RefCell<Option<Box<dyn Fn()>>>,
    /// Checklists: pending re-scan, pointer over a task box, completion popup shown.
    task_timer: RefCell<Option<glib::SourceId>>,
    task_hover: Cell<bool>,
    completion_open: Cell<bool>,
    /// Extra right-click menu entries: in general and with the cursor in a table.
    context_menu: RefCell<Option<gio::MenuModel>>,
    table_menu: RefCell<Option<gio::MenuModel>>,
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
        // Tab moves between the fields of inserted (Emmet) snippets; GtkSourceView's
        // own snippet files are not loaded, see `main.rs`.
        view.set_enable_snippets(true);
        view.set_top_margin(TOP_MARGIN);
        view.set_bottom_margin(120);
        view.set_left_margin(LEFT_MARGIN);
        view.set_right_margin(RIGHT_MARGIN);
        view.add_css_class("editor-view");
        let completion = view.completion();
        completion.set_show_icons(false);
        completion.add_provider(&crate::completion::SlidesProvider::default());

        let scrolled = gtk::ScrolledWindow::builder()
            .child(&view)
            .hexpand(true)
            .vexpand(true)
            .build();

        let format_bar = format_bar::build();
        let source = gtk::Box::new(gtk::Orientation::Vertical, 0);
        source.append(&format_bar);
        source.append(&scrolled);

        let root = gtk::Paned::new(gtk::Orientation::Horizontal);
        root.set_start_child(Some(&source));
        root.set_shrink_start_child(false);
        root.set_shrink_end_child(false);
        root.set_widget_name(&id);

        // Password prompt of a locked tab.
        let lock_entry = gtk::PasswordEntry::builder().show_peek_icon(true).placeholder_text(t!("Password")).build();
        let lock_button = gtk::Button::builder().label(t!("Unlock")).halign(gtk::Align::Center).build();
        lock_button.add_css_class("suggested-action");
        lock_button.add_css_class("pill");
        let lock_error = gtk::Label::builder().wrap(true).justify(gtk::Justification::Center).build();
        lock_error.add_css_class("error");
        let column = gtk::Box::new(gtk::Orientation::Vertical, 12);
        column.set_width_request(280);
        column.set_halign(gtk::Align::Center);
        column.append(&lock_entry);
        column.append(&lock_error);
        column.append(&lock_button);
        let lock_page = adw::StatusPage::builder()
            .icon_name("system-lock-screen-symbolic")
            .title(t!("Locked Note"))
            .description(t!("This note is protected with a password."))
            .child(&column)
            .build();
        let frame = gtk::Stack::new();
        frame.add_named(&root, Some("editor"));
        frame.add_named(&lock_page, Some("locked"));
        frame.set_visible_child_name("editor");
        frame.set_widget_name(&id);

        let tab = Rc::new(Self {
            id,
            frame,
            root,
            format_bar,
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
            focus: Cell::new(false),
            split_max: Cell::new(-1),
            split: Cell::new(None),
            preview_timer: RefCell::new(None),
            rendering: Cell::new(false),
            rerender: Cell::new(false),
            search: RefCell::new(None),
            webview_hook: RefCell::new(None),
            message_hook: RefCell::new(None),
            calc: RefCell::new(None),
            calc_timer: RefCell::new(None),
            slide_lines: RefCell::new(Vec::new()),
            shown_slide: Cell::new(0),
            snippet: RefCell::new(None),
            tab_lock: RefCell::new(None),
            lock_entry: lock_entry.clone(),
            lock_error,
            lock_hook: RefCell::new(None),
            task_timer: RefCell::new(None),
            task_hover: Cell::new(false),
            completion_open: Cell::new(false),
            context_menu: RefCell::new(None),
            table_menu: RefCell::new(None),
        });
        let weak = Rc::downgrade(&tab);
        lock_entry.connect_activate(move |_| {
            if let Some(t) = weak.upgrade() {
                t.unlock();
            }
        });
        let weak = Rc::downgrade(&tab);
        lock_button.connect_clicked(move |_| {
            if let Some(t) = weak.upgrade() {
                t.unlock();
            }
        });
        tab.buffer.create_tag(Some(TASK_DONE_TAG), &[("strikethrough", &true)]);
        tab.buffer.create_tag(Some(TASK_BOX_TAG), &[("weight", &700)]);

        let weak = Rc::downgrade(&tab);
        tab.buffer.connect_changed(move |_| {
            if let Some(t) = weak.upgrade() {
                if !t.loading.get() {
                    if let Some(kind) = t.preview_kind() {
                        t.schedule_render(kind.render_delay_ms());
                    }
                    if t.is_calc() {
                        t.schedule_calc();
                    }
                    t.schedule_tasks();
                }
            }
        });

        // The presentation preview follows the cursor.
        let weak = Rc::downgrade(&tab);
        tab.buffer.connect_notify_local(Some("cursor-position"), move |_, _| {
            if let Some(t) = weak.upgrade() {
                t.follow_cursor();
            }
        });

        // The split keeps its ratio when the pane is resized; moving the
        // divider changes the ratio.
        for prop in ["max-position", "position"] {
            let weak = Rc::downgrade(&tab);
            tab.root.connect_notify_local(Some(prop), move |_, _| {
                if let Some(t) = weak.upgrade() {
                    t.split_changed();
                }
            });
        }

        // Tab expands Emmet abbreviations before the cursor (HTML, CSS, …).
        let keys = gtk::EventControllerKey::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = Rc::downgrade(&tab);
        keys.connect_key_pressed(move |_, key, _, mods| {
            let modifiers = gdk::ModifierType::CONTROL_MASK
                | gdk::ModifierType::ALT_MASK
                | gdk::ModifierType::SHIFT_MASK
                | gdk::ModifierType::SUPER_MASK;
            let Some(t) = weak.upgrade() else { return glib::Propagation::Proceed };
            // Tables: Tab / Shift+Tab move between cells, Enter to the next row.
            let table_op = match key {
                gdk::Key::Tab if !mods.intersects(modifiers) => Some(table::Op::NextCell),
                gdk::Key::ISO_Left_Tab | gdk::Key::Tab if mods.intersects(modifiers) && (mods & modifiers) == gdk::ModifierType::SHIFT_MASK => {
                    Some(table::Op::PrevCell)
                }
                gdk::Key::Return | gdk::Key::KP_Enter if !mods.intersects(modifiers) => Some(table::Op::NextRow),
                _ => None,
            };
            if let Some(op) = table_op {
                let one_line = t.buffer.selection_bounds().is_none_or(|(s, e)| s.line() == e.line());
                if one_line && !t.snippet_active() && !t.completion_open.get() && t.table_op(op) {
                    return glib::Propagation::Stop;
                }
            }
            if mods.intersects(modifiers) {
                return glib::Propagation::Proceed;
            }
            if matches!(key, gdk::Key::Return | gdk::Key::KP_Enter) {
                return match weak.upgrade() {
                    Some(t) if t.continue_list() => glib::Propagation::Stop,
                    _ => glib::Propagation::Proceed,
                };
            }
            if key != gdk::Key::Tab {
                return glib::Propagation::Proceed;
            }
            match weak.upgrade() {
                Some(t) if t.shared.emmet_tab.get() && !t.snippet_active() && t.emmet_expand(true) => {
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });
        tab.view.add_controller(keys);

        // Checklists: a click on `[ ]` toggles the task.
        let click = gtk::GestureClick::new();
        click.set_button(gdk::BUTTON_PRIMARY);
        click.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = Rc::downgrade(&tab);
        click.connect_pressed(move |g, n, x, y| {
            let Some(t) = weak.upgrade() else { return };
            let mods = g.current_event_state();
            if n != 1 || mods.intersects(gdk::ModifierType::SHIFT_MASK | gdk::ModifierType::CONTROL_MASK) {
                return;
            }
            if let Some(line) = t.task_box_at(x, y) {
                g.set_state(gtk::EventSequenceState::Claimed);
                t.toggle_task(line);
            }
        });
        tab.view.add_controller(click);
        // A right click moves the cursor (unless it hits the selection), so the
        // context menu's table commands act on the clicked cell.
        let right = gtk::GestureClick::new();
        right.set_button(gdk::BUTTON_SECONDARY);
        right.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = Rc::downgrade(&tab);
        right.connect_pressed(move |g, _, x, y| {
            let Some(t) = weak.upgrade() else { return };
            let (bx, by) = t.view.window_to_buffer_coords(gtk::TextWindowType::Widget, x as i32, y as i32);
            // Right of the text: end of that line.
            let iter = t.view.iter_at_location(bx, by).unwrap_or_else(|| {
                let (mut it, _) = t.view.line_at_y(by);
                if !it.ends_line() {
                    it.forward_to_line_end();
                }
                it
            });
            let in_selection = t.buffer.selection_bounds().is_some_and(|(s, e)| iter.in_range(&s, &e));
            if !in_selection {
                t.buffer.place_cursor(&iter);
            }
            g.set_state(gtk::EventSequenceState::Claimed);
            t.popup_context_menu(x as i32, y as i32);
        });
        tab.view.add_controller(right);
        // Keyboard: the same menu at the cursor.
        let keys = gtk::ShortcutController::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = Rc::downgrade(&tab);
        keys.add_shortcut(gtk::Shortcut::new(
            gtk::ShortcutTrigger::parse_string("<Shift>F10|Menu"),
            Some(gtk::CallbackAction::new(move |_, _| {
                let Some(t) = weak.upgrade() else { return glib::Propagation::Proceed };
                let rect = t.view.iter_location(&t.buffer.iter_at_mark(&t.buffer.get_insert()));
                let (x, y) = t.view.buffer_to_window_coords(gtk::TextWindowType::Widget, rect.x(), rect.y() + rect.height());
                t.popup_context_menu(x, y);
                glib::Propagation::Stop
            })),
        ));
        tab.view.add_controller(keys);
        let motion = gtk::EventControllerMotion::new();
        let weak = Rc::downgrade(&tab);
        motion.connect_motion(move |_, x, y| {
            let Some(t) = weak.upgrade() else { return };
            let over = t.task_box_at(x, y).is_some();
            if t.task_hover.replace(over) != over {
                t.view.set_cursor_from_name(Some(if over { "pointer" } else { "text" }));
            }
        });
        let weak = Rc::downgrade(&tab);
        motion.connect_leave(move |_| {
            if let Some(t) = weak.upgrade() {
                if t.task_hover.replace(false) {
                    t.view.set_cursor_from_name(Some("text"));
                }
            }
        });
        tab.view.add_controller(motion);
        let weak = Rc::downgrade(&tab);
        completion.connect_show(move |_| {
            if let Some(t) = weak.upgrade() {
                t.completion_open.set(true);
            }
        });
        let weak = Rc::downgrade(&tab);
        completion.connect_hide(move |_| {
            if let Some(t) = weak.upgrade() {
                t.completion_open.set(false);
            }
        });

        // Distraction-free mode centres the text; follow the visible width.
        let weak = Rc::downgrade(&tab);
        scrolled.hadjustment().connect_page_size_notify(move |_| {
            if let Some(t) = weak.upgrade() {
                t.update_focus_margins();
            }
        });

        tab.apply_theme();
        tab
    }

    // ----- Emmet ---------------------------------------------------------

    /// Emmet syntax of this tab; `None` for languages without Emmet.
    pub fn emmet_syntax(&self) -> Option<emmet::Syntax> {
        emmet::syntax_for(self.language_id().as_deref())
    }

    /// Expands the Emmet abbreviation before the cursor. With `from_tab`,
    /// plain words that are no abbreviations are left alone.
    pub fn emmet_expand(&self, from_tab: bool) -> bool {
        let Some(document) = self.emmet_syntax() else { return false };
        if self.buffer.has_selection() || self.large.get() {
            return false;
        }
        let cursor = self.buffer.iter_at_mark(&self.buffer.get_insert());
        let mut line_start = cursor;
        line_start.set_line_offset(0);
        let line = self.buffer.text(&line_start, &cursor, false).to_string();
        // Context (inside <style>, a rule block, …) from the text before the cursor.
        let mut context_start = cursor;
        context_start.backward_chars(200_000);
        let before = self.buffer.text(&context_start, &cursor, false).to_string();
        let Some(syntax) = emmet::syntax_at(&before, document) else { return false };
        let Some(found) = emmet::extract(&line, syntax) else { return false };
        if from_tab && !emmet::tab_expands(&found, syntax, &before) {
            return false;
        }
        // The snippet takes over the line's indentation and tab/space setting.
        let Ok(text) = emmet::expand(&found.abbreviation, syntax, "\t", "", None) else { return false };
        let mut start = line_start;
        start.forward_chars(found.start as i32);
        self.insert_snippet(&text, start, cursor)
    }

    /// Wraps the selection (or the current line) with an Emmet abbreviation.
    pub fn emmet_wrap(&self, abbreviation: &str) -> Result<(), String> {
        let syntax = self.emmet_syntax().filter(|s| !s.stylesheet).ok_or_else(|| t!("Not available here").to_owned())?;
        let (start, end) = match self.buffer.selection_bounds() {
            Some((mut start, end)) => {
                // A selection starting in the indentation starts at the text, so
                // the wrapper keeps the indentation of the line.
                let mut line_start = start;
                line_start.set_line_offset(0);
                if self.buffer.text(&line_start, &start, false).trim().is_empty() {
                    while start < end && !start.ends_line() && start.char().is_whitespace() {
                        start.forward_char();
                    }
                }
                (start, end)
            }
            None => {
                let mut start = self.buffer.iter_at_mark(&self.buffer.get_insert());
                start.set_line_offset(0);
                while !start.ends_line() && start.char().is_whitespace() {
                    start.forward_char();
                }
                let mut end = start;
                if !end.ends_line() {
                    end.forward_to_line_end();
                }
                (start, end)
            }
        };
        // Indentation before the selection counts as part of the first line, so
        // the common indentation is the same for all lines.
        let mut line_start = start;
        line_start.set_line_offset(0);
        let prefix = self.buffer.text(&line_start, &start, false).to_string();
        let prefix = if prefix.trim().is_empty() { prefix } else { String::new() };
        let text = format!("{prefix}{}", self.buffer.text(&start, &end, false));
        // Wrapped lines lose their common indentation; Emmet indents them again.
        let indent = text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start().len())
            .min()
            .unwrap_or(0);
        let lines: Vec<&str> = text.lines().map(|l| l.get(indent..).unwrap_or(l.trim_start())).collect();
        let lines: Vec<&str> = if lines.is_empty() { vec![""] } else { lines };
        let out = emmet::expand(abbreviation, syntax, "\t", "", Some(&lines))?;
        if self.insert_snippet(&out, start, end) {
            Ok(())
        } else {
            Err(t!("Invalid abbreviation").to_owned())
        }
    }

    /// Replaces `start..end` with a GtkSourceView snippet (tab stops active).
    fn insert_snippet(&self, text: &str, mut start: gtk::TextIter, mut end: gtk::TextIter) -> bool {
        let Ok(snippet) = sourceview5::Snippet::new_parsed(text) else { return false };
        self.buffer.begin_user_action();
        self.buffer.delete(&mut start, &mut end);
        self.view.push_snippet(&snippet, Some(&mut start));
        self.buffer.end_user_action();
        *self.snippet.borrow_mut() = Some(snippet);
        true
    }

    /// An inserted snippet still has fields to move to (GtkSourceView
    /// detaches a snippet from the buffer once it is finished).
    fn snippet_active(&self) -> bool {
        let mut slot = self.snippet.borrow_mut();
        let active = slot.as_ref().is_some_and(|s| s.buffer().is_some() && s.focus_position() >= 0);
        if !active {
            slot.take();
        }
        active
    }

    // ----- tab password -------------------------------------------------

    pub fn lock_state(&self) -> LockState {
        match self.tab_lock.borrow().as_ref() {
            None => LockState::Open,
            Some(l) if l.key.is_some() => LockState::Unlocked,
            Some(_) => LockState::Locked,
        }
    }

    pub fn lock(&self) -> Option<crypto::Lock> {
        self.tab_lock.borrow().as_ref().map(|l| l.lock.clone())
    }

    /// The tab's key while unlocked.
    pub fn tab_key(&self) -> Option<crypto::Key> {
        self.tab_lock.borrow().as_ref().and_then(|l| l.key.clone())
    }

    fn lock_changed(&self) {
        if let Some(hook) = self.lock_hook.borrow().as_ref() {
            hook();
        }
    }

    /// Restores a locked tab from its sealed content.
    pub fn set_locked(&self, lock: crypto::Lock, sealed: Vec<u8>) {
        *self.tab_lock.borrow_mut() = Some(TabLock { lock, key: None, sealed });
        self.set_text_irreversible("");
        self.frame.set_visible_child_name("locked");
    }

    /// Protects the tab with a password (it stays unlocked).
    pub fn protect(&self, lock: crypto::Lock, key: crypto::Key) {
        *self.tab_lock.borrow_mut() = Some(TabLock { lock, key: Some(key), sealed: Vec::new() });
        self.lock_changed();
    }

    /// New password, same key.
    pub fn set_lock(&self, lock: crypto::Lock) {
        if let Some(l) = self.tab_lock.borrow_mut().as_mut() {
            l.lock = lock;
        }
        self.lock_changed();
    }

    pub fn unprotect(&self) {
        if self.lock_state() == LockState::Unlocked {
            self.tab_lock.borrow_mut().take();
            self.lock_changed();
        }
    }

    /// The content as stored for a protected tab.
    pub fn sealed_content(&self) -> Option<Vec<u8>> {
        let lock = self.tab_lock.borrow();
        let l = lock.as_ref()?;
        Some(match &l.key {
            Some(key) => key.seal(self.text().as_bytes(), TAB_AAD),
            None => l.sealed.clone(),
        })
    }

    /// Hides the content again.
    pub fn lock_now(&self) {
        let Some(sealed) = self.sealed_content() else { return };
        if let Some(l) = self.tab_lock.borrow_mut().as_mut() {
            l.key = None;
            l.sealed = sealed;
        }
        self.set_text_irreversible("");
        self.lock_error.set_text("");
        self.frame.set_visible_child_name("locked");
        self.lock_changed();
    }

    /// Checks the password from the prompt and shows the content.
    fn unlock(self: &Rc<Self>) {
        let password = Zeroizing::new(self.lock_entry.text().to_string());
        let Some((lock, sealed)) = self.tab_lock.borrow().as_ref().map(|l| (l.lock.clone(), l.sealed.clone())) else {
            return;
        };
        if password.is_empty() || !self.lock_entry.is_sensitive() {
            return;
        }
        self.lock_entry.set_sensitive(false);
        let weak = Rc::downgrade(self);
        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(move || {
                let key = lock.open(&password)?;
                let plain = key.open(&sealed, TAB_AAD)?;
                Ok::<_, crypto::Error>((key, String::from_utf8_lossy(&plain).into_owned()))
            })
            .await;
            let Some(t) = weak.upgrade() else { return };
            t.lock_entry.set_sensitive(true);
            match result {
                Ok(Ok((key, text))) => {
                    t.lock_entry.set_text("");
                    t.lock_error.set_text("");
                    if let Some(l) = t.tab_lock.borrow_mut().as_mut() {
                        l.key = Some(key);
                        l.sealed.clear();
                    }
                    t.set_text_irreversible(&text);
                    t.update_tasks();
                    t.frame.set_visible_child_name("editor");
                    t.schedule_render(0);
                    t.lock_changed();
                    t.view.grab_focus();
                }
                Ok(Err(e)) => {
                    t.lock_error.set_text(&e.to_string());
                    t.lock_entry.grab_focus();
                }
                Err(_) => {}
            }
        });
    }

    // ----- checklists ----------------------------------------------------

    /// Task items are recognised in plain notes, Markdown and presentations.
    pub fn tasks_enabled(&self) -> bool {
        !self.large.get() && matches!(self.language_id().as_deref(), None | Some("markdown") | Some(slides::LANGUAGE))
    }

    fn line_text(&self, line: i32) -> Option<String> {
        let start = self.buffer.iter_at_line(line)?;
        let mut end = start;
        if !end.ends_line() {
            end.forward_to_line_end();
        }
        Some(self.buffer.text(&start, &end, false).into())
    }

    /// Line whose task box is under the widget position `x`, `y`.
    fn task_box_at(&self, x: f64, y: f64) -> Option<i32> {
        if !self.tasks_enabled() {
            return None;
        }
        let (bx, by) = self.view.window_to_buffer_coords(gtk::TextWindowType::Widget, x as i32, y as i32);
        let iter = self.view.iter_at_location(bx, by)?;
        let task = checklist::parse(&self.line_text(iter.line())?)?;
        let offset = iter.line_offset() as usize;
        (task.box_start..task.box_start + 3).contains(&offset).then_some(iter.line())
    }

    // ----- context menu --------------------------------------------------

    /// Sets the extra right-click entries; `table` replaces them while the
    /// cursor is in a table.
    pub fn set_context_menus(&self, general: gio::MenuModel, table: Option<gio::MenuModel>) {
        *self.context_menu.borrow_mut() = Some(general);
        *self.table_menu.borrow_mut() = table;
    }

    /// Own right-click menu instead of GtkTextView's, so submenus open beside
    /// it instead of replacing it.
    fn popup_context_menu(&self, x: i32, y: i32) {
        let view = &self.view;
        let has_selection = self.buffer.has_selection();
        let editable = view.is_editable();
        let can_paste = view.clipboard().formats().contains_type(glib::Type::STRING);
        view.action_set_enabled("clipboard.cut", editable && has_selection);
        view.action_set_enabled("clipboard.copy", has_selection);
        view.action_set_enabled("clipboard.paste", editable && can_paste);
        view.action_set_enabled("selection.delete", editable && has_selection);
        view.action_set_enabled("selection.select-all", self.buffer.char_count() > 0);

        let menu = gio::Menu::new();
        let clipboard = gio::Menu::new();
        clipboard.append(Some(t!("Cut")), Some("clipboard.cut"));
        clipboard.append(Some(t!("Copy")), Some("clipboard.copy"));
        clipboard.append(Some(t!("Paste")), Some("clipboard.paste"));
        clipboard.append(Some(t!("Delete")), Some("selection.delete"));
        menu.append_section(None, &clipboard);
        // In a table only the table commands, besides the clipboard.
        let table = self.table_at_cursor().and_then(|_| self.table_menu.borrow().clone());
        if let Some(table) = table {
            menu.append_section(None, &table);
        } else {
            let more = gio::Menu::new();
            more.append(Some(t!("Select All")), Some("selection.select-all"));
            more.append(Some(t!("Insert Emoji")), Some("misc.insert-emoji"));
            menu.append_section(None, &more);
            if let Some(extra) = self.context_menu.borrow().clone() {
                menu.append_section(None, &extra);
            }
        }

        let popover = gtk::PopoverMenu::from_model_full(&menu, gtk::PopoverMenuFlags::NESTED);
        popover.set_parent(view);
        popover.set_has_arrow(false);
        popover.set_halign(gtk::Align::Start);
        popover.set_position(gtk::PositionType::Bottom);
        popover.set_pointing_to(Some(&gdk::Rectangle::new(x, y, 1, 1)));
        popover.connect_closed(|p| {
            let p = p.clone();
            glib::idle_add_local_once(move || p.unparent());
        });
        popover.popup();
    }

    // ----- Markdown formatting --------------------------------------------

    /// Applies a formatting command to the selection (or the cursor's word / line).
    pub fn md_format(&self, op: md_format::Op) {
        let buf = &self.buffer;
        let (s, e) = buf.selection_bounds().unwrap_or_else(|| {
            let it = buf.iter_at_mark(&buf.get_insert());
            (it, it)
        });
        let mut last = e.line();
        if last > s.line() && e.starts_line() {
            last -= 1;
        }
        let Some(start) = buf.iter_at_line(s.line()) else { return };
        let Some(mut end) = buf.iter_at_line(last) else { return };
        if !end.ends_line() {
            end.forward_to_line_end();
        }
        let old = buf.text(&start, &end, true).to_string();
        let to_byte = |chars: i32| old.char_indices().nth(chars.max(0) as usize).map_or(old.len(), |(i, _)| i);
        let sel = (to_byte(s.offset() - start.offset()), to_byte(e.offset().min(end.offset()) - start.offset()));
        let edit = md_format::apply(&old, sel, op);

        let base = start.offset();
        if edit.text != old {
            let (mut start, mut end) = (start, end);
            buf.begin_user_action();
            buf.delete(&mut start, &mut end);
            buf.insert(&mut start, &edit.text);
            buf.end_user_action();
        }
        let to_char = |byte: usize| edit.text[..byte].chars().count() as i32;
        let a = buf.iter_at_offset(base + to_char(edit.sel.0));
        let b = buf.iter_at_offset(base + to_char(edit.sel.1));
        buf.select_range(&b, &a);
        self.view.scroll_mark_onscreen(&buf.get_insert());
        self.view.grab_focus();
    }

    // ----- tables --------------------------------------------------------

    /// First and last line of the table at the cursor.
    pub fn table_at_cursor(&self) -> Option<(i32, i32)> {
        if !self.tasks_enabled() {
            return None;
        }
        let line = self.buffer.iter_at_mark(&self.buffer.get_insert()).line();
        let is_table = |l: i32| self.line_text(l).is_some_and(|t| table::is_table_line(&t));
        if !is_table(line) {
            return None;
        }
        let (mut first, mut last) = (line, line);
        while first > 0 && is_table(first - 1) {
            first -= 1;
        }
        while last + 1 < self.buffer.line_count() && is_table(last + 1) {
            last += 1;
        }
        Some((first, last))
    }

    /// Applies a table operation at the cursor; `false` outside tables.
    pub fn table_op(&self, op: table::Op) -> bool {
        let Some((first, last)) = self.table_at_cursor() else { return false };
        let cursor = self.buffer.iter_at_mark(&self.buffer.get_insert());
        let old: Vec<String> = (first..=last).filter_map(|l| self.line_text(l)).collect();
        let refs: Vec<&str> = old.iter().map(String::as_str).collect();
        let Some(edit) = table::apply(&refs, (cursor.line() - first) as usize, cursor.line_offset() as usize, op) else {
            return true;
        };
        if edit.lines != old {
            let Some(mut start) = self.buffer.iter_at_line(first) else { return true };
            let Some(mut end) = self.buffer.iter_at_line(last) else { return true };
            if !end.ends_line() {
                end.forward_to_line_end();
            }
            self.buffer.begin_user_action();
            self.buffer.delete(&mut start, &mut end);
            self.buffer.insert(&mut start, &edit.lines.join("\n"));
            self.buffer.end_user_action();
        }
        let line = first + edit.cursor_line as i32;
        if let Some(start) = self.buffer.iter_at_line_offset(line, edit.cursor_col as i32) {
            let mut end = start;
            end.forward_chars(edit.select as i32);
            self.buffer.select_range(&end, &start);
            self.view.scroll_mark_onscreen(&self.buffer.get_insert());
        }
        true
    }

    /// Inserts an empty table below the cursor line.
    pub fn insert_table(&self) {
        let cursor = self.buffer.iter_at_mark(&self.buffer.get_insert());
        let line = cursor.line();
        let current = self.line_text(line).unwrap_or_default();
        let indent: String = current.chars().take_while(|c| *c == ' ' || *c == '\t').collect();
        let edit = table::template(3, &indent);
        let (mut at, prefix, first) = if current.trim().is_empty() {
            let mut start = cursor;
            start.set_line_offset(0);
            let mut end = start;
            if !end.ends_line() {
                end.forward_to_line_end();
            }
            self.buffer.begin_user_action();
            self.buffer.delete(&mut start, &mut end);
            (start, "", line)
        } else {
            let mut end = cursor;
            if !end.ends_line() {
                end.forward_to_line_end();
            }
            self.buffer.begin_user_action();
            (end, "\n\n", line + 2)
        };
        self.buffer.insert(&mut at, &format!("{prefix}{}", edit.lines.join("\n")));
        self.buffer.end_user_action();
        if let Some(iter) = self.buffer.iter_at_line_offset(first + edit.cursor_line as i32, edit.cursor_col as i32) {
            self.buffer.place_cursor(&iter);
            self.view.scroll_mark_onscreen(&self.buffer.get_insert());
        }
    }

    /// Toggles the task on `line` (also used by clicks in the preview).
    pub fn toggle_task(&self, line: i32) {
        let Some(text) = self.line_text(line) else { return };
        let Some(new) = checklist::toggle(&text) else { return };
        self.replace_line(line, &text, &new);
    }

    /// Replaces a line, touching only the part that differs so the cursor,
    /// selection and undo history stay as precise as possible.
    fn replace_line(&self, line: i32, old: &str, new: &str) {
        if old == new {
            return;
        }
        let (old_c, new_c): (Vec<char>, Vec<char>) = (old.chars().collect(), new.chars().collect());
        let prefix = old_c.iter().zip(&new_c).take_while(|(a, b)| a == b).count();
        let max_suffix = old_c.len().min(new_c.len()) - prefix;
        let suffix = old_c.iter().rev().zip(new_c.iter().rev()).take(max_suffix).take_while(|(a, b)| a == b).count();
        let Some(mut start) = self.buffer.iter_at_line_offset(line, prefix as i32) else { return };
        let Some(mut end) = self.buffer.iter_at_line_offset(line, (old_c.len() - suffix) as i32) else { return };
        let insert: String = new_c[prefix..new_c.len() - suffix].iter().collect();
        self.buffer.begin_user_action();
        self.buffer.delete(&mut start, &mut end);
        self.buffer.insert(&mut start, &insert);
        self.buffer.end_user_action();
    }

    /// Ctrl+Enter: checks/unchecks the tasks on the selected lines (or the
    /// cursor line), or turns them into tasks.
    pub fn toggle_tasks_in_selection(&self) {
        let (start, end) = match self.buffer.selection_bounds() {
            Some((s, mut e)) => {
                // A selection ending at the start of a line does not include it.
                if e.line() > s.line() && e.starts_line() {
                    e.backward_char();
                }
                (s.line(), e.line())
            }
            None => {
                let line = self.buffer.iter_at_mark(&self.buffer.get_insert()).line();
                (line, line)
            }
        };
        let old: Vec<String> = (start..=end).filter_map(|l| self.line_text(l)).collect();
        let refs: Vec<&str> = old.iter().map(String::as_str).collect();
        let new = checklist::toggle_lines(&refs);
        self.buffer.begin_user_action();
        for (i, (o, n)) in old.iter().zip(&new).enumerate() {
            self.replace_line(start + i as i32, o, n);
        }
        self.buffer.end_user_action();
    }

    /// Enter in a list item starts the next item (an unchecked task after a
    /// task); Enter in an empty item ends the list.
    fn continue_list(&self) -> bool {
        if !self.tasks_enabled() || self.buffer.has_selection() || self.completion_open.get() || self.snippet_active() {
            return false;
        }
        let cursor = self.buffer.iter_at_mark(&self.buffer.get_insert());
        let mut line_start = cursor;
        line_start.set_line_offset(0);
        let before = self.buffer.text(&line_start, &cursor, false).to_string();
        let Some(next) = checklist::continuation(&before) else { return false };
        let mut line_end = cursor;
        if !line_end.ends_line() {
            line_end.forward_to_line_end();
        }
        let after = self.buffer.text(&cursor, &line_end, false);
        self.buffer.begin_user_action();
        match next {
            checklist::Continue::Item(prefix) => {
                let mut at = cursor;
                self.buffer.insert(&mut at, &format!("\n{prefix}"));
            }
            checklist::Continue::End if after.trim().is_empty() => {
                let (mut s, mut e) = (line_start, line_end);
                self.buffer.delete(&mut s, &mut e);
            }
            checklist::Continue::End => {
                self.buffer.end_user_action();
                return false;
            }
        }
        self.buffer.end_user_action();
        self.view.scroll_mark_onscreen(&self.buffer.get_insert());
        true
    }

    fn schedule_tasks(self: &Rc<Self>) {
        if let Some(id) = self.task_timer.borrow_mut().take() {
            id.remove();
        }
        let weak = Rc::downgrade(self);
        let id = glib::timeout_add_local_once(Duration::from_millis(120), move || {
            if let Some(t) = weak.upgrade() {
                t.task_timer.borrow_mut().take();
                t.update_tasks();
            }
        });
        *self.task_timer.borrow_mut() = Some(id);
    }

    /// Strikes through done tasks and emphasises the boxes.
    fn update_tasks(&self) {
        let (start, end) = self.buffer.bounds();
        self.buffer.remove_tag_by_name(TASK_DONE_TAG, &start, &end);
        self.buffer.remove_tag_by_name(TASK_BOX_TAG, &start, &end);
        if !self.tasks_enabled() {
            return;
        }
        let text = self.text();
        for (line, content) in text.lines().enumerate() {
            let Some(task) = checklist::parse(content) else { continue };
            let line = line as i32;
            let at = |offset: usize| self.buffer.iter_at_line_offset(line, offset as i32);
            if let (Some(s), Some(e)) = (at(task.box_start), at(task.box_start + 3)) {
                self.buffer.apply_tag_by_name(TASK_BOX_TAG, &s, &e);
            }
            if task.checked {
                if let (Some(s), Some(mut e)) = (at(task.text_start), at(task.text_start)) {
                    if !e.ends_line() {
                        e.forward_to_line_end();
                    }
                    self.buffer.apply_tag_by_name(TASK_DONE_TAG, &s, &e);
                }
            }
        }
    }

    /// Done tasks are dimmed with the theme's text colour.
    fn apply_task_style(&self) {
        let theme = self.shared.theme.get();
        let fg = theme.editor.as_ref().map(|e| e.fg.as_str()).unwrap_or(if theme.dark { "#ffffff" } else { "#000000" });
        let mut rgba = gdk::RGBA::parse(fg).unwrap_or(gdk::RGBA::BLACK);
        rgba.set_alpha(0.45);
        if let Some(tag) = self.buffer.tag_table().lookup(TASK_DONE_TAG) {
            tag.set_foreground_rgba(Some(&rgba));
        }
    }

    // ----- distraction-free mode ----------------------------------------

    pub fn set_focus_mode(self: &Rc<Self>, on: bool) {
        if self.focus.replace(on) == on {
            return;
        }
        self.view.set_show_line_numbers(!on);
        self.view.set_highlight_current_line(!on);
        self.view.set_top_margin(if on { FOCUS_TOP_MARGIN } else { TOP_MARGIN });
        self.sync_preview();
        self.update_focus_margins();
    }

    /// Side margins: centred column in distraction-free mode, defaults otherwise.
    fn update_focus_margins(&self) {
        let (left, right) = if self.focus.get() {
            let width = self.view.hadjustment().map_or(0.0, |a| a.page_size()) as i32;
            let char_width = self.view.create_pango_layout(Some("0")).pixel_size().0.max(1);
            let side = ((width - FOCUS_COLUMNS * char_width) / 2).max(LEFT_MARGIN);
            (side, side.max(RIGHT_MARGIN))
        } else {
            (LEFT_MARGIN, RIGHT_MARGIN)
        };
        if self.view.left_margin() != left {
            self.view.set_left_margin(left);
        }
        if self.view.right_margin() != right {
            self.view.set_right_margin(right);
        }
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
        self.recalc();
        self.update_tasks();
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
        if iter.is_start() {
            return;
        }
        // Scroll only as far as needed to make the cursor visible.
        let view = self.view.clone();
        let buffer = self.buffer.clone();
        glib::idle_add_local_once(move || {
            view.scroll_to_mark(&buffer.get_insert(), 0.0, false, 0.0, 0.0);
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
        self.recalc();
        self.update_tasks();
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
        if self.lock_state() == LockState::Locked {
            return t!("Locked note").into();
        }
        if self.is_kanban() {
            if let Some(title) = kanban::title(&self.text()) {
                return truncate(&title, 32);
            }
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
                    return t!("{} snippet", lang.name());
                }
            }
            if !iter.forward_line() {
                break;
            }
        }
        t!("Untitled").into()
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
        self.sync_calc();
        self.update_tasks();
    }

    // ----- calculations --------------------------------------------------

    pub fn is_calc(&self) -> bool {
        self.language_id().as_deref() == Some(CALC_LANGUAGE)
    }

    fn message(&self, text: &str) {
        if let Some(hook) = self.message_hook.borrow().as_ref() {
            hook(text);
        }
    }

    /// Shows or hides the result column depending on the language. The
    /// column is created once and only hidden afterwards: removing a gutter
    /// renderer releases it twice in GtkSourceView 5.12.
    fn sync_calc(self: &Rc<Self>) {
        let gutter = sourceview5::prelude::ViewExt::gutter(&self.view, gtk::TextWindowType::Right);
        if !self.is_calc() {
            if let Some(column) = self.calc.borrow().as_ref() {
                column.results.borrow_mut().clear();
                column.renderer.set_visible(false);
            }
            return;
        }
        if let Some(column) = self.calc.borrow().as_ref() {
            column.renderer.set_visible(true);
        }
        if self.calc.borrow().is_none() {
            let results: Rc<RefCell<Vec<LineResult>>> = Rc::default();
            let renderer = sourceview5::GutterRendererText::new();
            renderer.set_xpad(CALC_PAD);
            renderer.set_alignment_mode(sourceview5::GutterRendererAlignmentMode::First);
            renderer.add_css_class("calc-results");

            let r = results.clone();
            renderer.connect_query_data(move |renderer, _lines, line| {
                let markup = r.borrow().get(line as usize).map(result_markup).unwrap_or_default();
                renderer.set_markup(&markup);
            });
            let r = results.clone();
            renderer.connect_query_activatable(move |_, iter, _| {
                r.borrow().get(iter.line() as usize).and_then(LineResult::display).is_some()
            });
            // Click on a result copies it.
            let r = results.clone();
            let weak = Rc::downgrade(self);
            renderer.connect_activate(move |renderer, iter, _, _, _, _| {
                let Some(text) = r.borrow().get(iter.line() as usize).and_then(LineResult::display) else {
                    return;
                };
                renderer.clipboard().set_text(&text);
                if let Some(t) = weak.upgrade() {
                    t.message(&t!("Copied “{}”", text));
                }
            });
            // Text metrics are only final once the column is shown.
            let weak = Rc::downgrade(self);
            renderer.connect_map(move |_| {
                if let Some(t) = weak.upgrade() {
                    t.fit_calc_column();
                }
            });
            gutter.insert(&renderer, 0);
            *self.calc.borrow_mut() = Some(CalcColumn { renderer, results });
        }
        self.recalc();
    }

    fn schedule_calc(self: &Rc<Self>) {
        if let Some(id) = self.calc_timer.borrow_mut().take() {
            id.remove();
        }
        let weak = Rc::downgrade(self);
        let id = glib::timeout_add_local_once(Duration::from_millis(120), move || {
            if let Some(t) = weak.upgrade() {
                t.calc_timer.borrow_mut().take();
                t.recalc();
            }
        });
        *self.calc_timer.borrow_mut() = Some(id);
    }

    fn recalc(&self) {
        if !self.is_calc() {
            return;
        }
        let calc = self.calc.borrow();
        let Some(column) = calc.as_ref() else { return };
        *column.results.borrow_mut() = calc::evaluate(&self.text());
        drop(calc);
        self.fit_calc_column();
    }

    /// Sizes the result column to the widest result.
    fn fit_calc_column(&self) {
        let calc = self.calc.borrow();
        let Some(column) = calc.as_ref() else { return };
        let width = column
            .results
            .borrow()
            .iter()
            .filter(|r| !matches!(r, LineResult::Empty))
            .map(|r| column.renderer.measure_markup(&result_markup(r)).0)
            .max()
            .map_or(0, |w| (w + 2 * CALC_PAD).min(900));
        column.renderer.set_size_request(width, -1);
        column.renderer.queue_resize();
        column.renderer.queue_draw();
    }

    pub fn set_language_id(self: &Rc<Self>, id: Option<&str>) {
        self.set_language(id.and_then(langs::by_id).as_ref());
    }

    pub fn apply_theme(&self) {
        let theme = self.shared.theme.get();
        let sm = sourceview5::StyleSchemeManager::default();
        let fallback = if theme.dark { ["Adwaita-dark", "oblivion"] } else { ["Adwaita", "classic"] };
        let scheme = sm
            .scheme(&theme.scheme_id())
            .or_else(|| sm.scheme(fallback[0]))
            .or_else(|| sm.scheme(fallback[1]));
        self.buffer.set_style_scheme(scheme.as_ref());
        self.apply_task_style();
        if let Some(p) = self.preview.borrow().as_ref() {
            if p.shell.get() != Shell::None {
                p.webview.set_background_color(&preview_bg(theme));
                let js = format!("setTheme('{}', '{}')", if theme.dark { "dark" } else { "light" }, theme.preview_vars());
                if p.loaded.get() {
                    p.webview.evaluate_javascript(&js, None, None, None::<&gio::Cancellable>, |_| {});
                }
            }
        }
    }

    /// Called after the editor font or its size changed.
    pub fn apply_zoom(self: &Rc<Self>) {
        if let Some(p) = self.preview.borrow().as_ref() {
            p.webview.set_zoom_level(self.shared.zoom.get());
        }
        // The new font is applied with the next style update.
        let weak = Rc::downgrade(self);
        glib::timeout_add_local_once(Duration::from_millis(150), move || {
            if let Some(t) = weak.upgrade() {
                t.fit_calc_column();
                t.update_focus_margins();
            }
        });
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
        self.is_kanban() || (self.preview_on.get() && self.preview_kind().is_some())
    }

    pub fn is_kanban(&self) -> bool {
        self.preview_kind() == Some(PreviewKind::Kanban)
    }

    /// State of the *Preview* toggle: for boards it shows the Markdown source.
    pub fn preview_toggle_state(&self) -> bool {
        if self.is_kanban() {
            self.preview_on.get()
        } else {
            self.preview_visible()
        }
    }

    pub fn set_preview(self: &Rc<Self>, on: bool) {
        self.preview_on.set(on);
        self.sync_preview();
    }

    /// Shows the formatting toolbar for Markdown and presentations (not in
    /// distraction-free mode).
    pub fn sync_format_bar(&self) {
        let markdown = matches!(self.preview_kind(), Some(PreviewKind::Markdown | PreviewKind::Slides));
        self.format_bar.set_visible(markdown && self.shared.format_bar.get() && !self.focus.get());
    }

    fn sync_preview(self: &Rc<Self>) {
        self.sync_format_bar();
        let kanban = self.is_kanban();
        if let Some(source) = self.root.start_child() {
            source.set_visible(!kanban || (self.preview_on.get() && !self.focus.get()));
        }
        if self.preview_visible() && (!self.focus.get() || kanban) {
            let webview = self.ensure_webview();
            if self.root.end_child().as_ref() != Some(webview.upcast_ref()) {
                self.root.set_end_child(Some(&webview));
                self.split_max.set(-1);
                self.split_changed();
            }
            self.schedule_render(0);
        } else if self.root.end_child().is_some() {
            self.root.set_end_child(None::<&gtk::Widget>);
        }
    }

    /// Applies the stored ratio when the pane size changed (window resized,
    /// preview shown again), otherwise remembers the ratio of the divider.
    fn split_changed(&self) {
        let max = self.root.max_position();
        // Unallocated panes report max-position = G_MAXINT.
        if self.root.end_child().is_none() || max <= 100 || max >= i32::MAX / 2 {
            return;
        }
        if max != self.split_max.get() {
            self.split_max.set(max);
            let position = (max as f64 * self.split.get().unwrap_or(0.5)).round() as i32;
            self.root.set_position(position);
        } else {
            self.split.set(Some((self.root.position() as f64 / max as f64).clamp(0.0, 1.0)));
        }
    }

    fn ensure_webview(self: &Rc<Self>) -> webkit6::WebView {
        if let Some(p) = self.preview.borrow().as_ref() {
            return p.webview.clone();
        }
        let theme = self.shared.theme.get();
        let webview = webkit6::WebView::new();
        webview.set_hexpand(true);
        webview.set_vexpand(true);
        webview.set_background_color(&preview_bg(theme));
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

        // Messages from the presentation preview ("go to slide").
        if let Some(ucm) = webview.user_content_manager() {
            ucm.register_script_message_handler("qn", None);
            let weak = Rc::downgrade(self);
            ucm.connect_script_message_received(Some("qn"), move |_, value| {
                let Some(t) = weak.upgrade() else { return };
                let Ok(msg) = serde_json::from_str::<serde_json::Value>(&value.to_str()) else { return };
                match msg.get("type").and_then(|v| v.as_str()) {
                    Some("goto") => {
                        if let Some(i) = msg.get("slide").and_then(|v| v.as_i64()) {
                            t.goto_slide(i);
                        }
                    }
                    Some("kanban") => {
                        if let Some(board) = msg.get("board") {
                            t.apply_board(board.clone());
                        }
                    }
                    Some("undo") if t.buffer.can_undo() => t.buffer.undo(),
                    Some("redo") if t.buffer.can_redo() => t.buffer.redo(),
                    Some("task") => {
                        if let Some(line) = msg.get("line").and_then(|v| v.as_i64()) {
                            t.toggle_task(line as i32);
                        }
                    }
                    _ => {}
                }
            });
        }

        *self.preview.borrow_mut() = Some(Preview {
            webview: webview.clone(),
            loaded: Cell::new(false),
            shell: Cell::new(Shell::None),
            pending: RefCell::new(None),
        });
        webview
    }

    /// Directory of the backing file (for relative links, images, includes).
    pub fn base_dir(&self) -> Option<PathBuf> {
        self.path.borrow().as_ref().and_then(|p| p.parent()).map(Path::to_path_buf)
    }

    pub fn base_uri(&self) -> Option<String> {
        self.base_dir().map(|d| format!("{}/", gio::File::for_path(d).uri()))
    }

    /// Base for relative paths in presentations: the file's folder, or the
    /// home folder for notes (so absolute paths work too).
    pub fn slides_base_uri(&self) -> Option<String> {
        self.base_uri()
            .or_else(|| dirs::home_dir().map(|d| format!("{}/", gio::File::for_path(d).uri())))
    }

    /// Loads a page into the preview.
    fn load_page(&self, p: &Preview, page: &str, shell: Shell) {
        p.loaded.set(false);
        p.shell.set(shell);
        let bg = if shell != Shell::None { preview_bg(self.shared.theme.get()) } else { gdk::RGBA::WHITE };
        p.webview.set_background_color(&bg);
        let base = if shell == Shell::Slides { self.slides_base_uri() } else { self.base_uri() };
        p.webview.load_html(page, base.as_deref());
    }

    /// The rendered document body (Markdown, reStructuredText), for export.
    pub fn document_html(&self) -> Option<String> {
        match self.preview_kind()? {
            PreviewKind::Markdown => Some(markdown::to_html(&self.text())),
            PreviewKind::Rst => Some(rst::to_html(&self.text(), self.base_dir().as_deref())),
            PreviewKind::PlantUml | PreviewKind::Html | PreviewKind::Slides | PreviewKind::Kanban => None,
        }
    }

    /// The parsed presentation, if this is one.
    pub fn deck(&self) -> Option<slides::Deck> {
        (self.preview_kind()? == PreviewKind::Slides).then(|| slides::parse(&self.text()))
    }

    /// Index of the slide at the cursor (0 when this is no presentation).
    pub fn current_slide(&self) -> usize {
        let line = self.buffer.iter_at_mark(&self.buffer.get_insert()).line().max(0) as usize;
        slides::slide_at(&slides::slide_lines(&self.text()), line)
    }

    /// Moves the cursor to the first line of slide `index`.
    fn goto_slide(&self, index: i64) {
        let lines = self.slide_lines.borrow().clone();
        if lines.is_empty() || index < 0 {
            return;
        }
        let (_, body) = lines[(index as usize).min(lines.len() - 1)];
        let iter = self.buffer.iter_at_line(body as i32).unwrap_or_else(|| self.buffer.end_iter());
        self.buffer.place_cursor(&iter);
        let mark = self.buffer.get_insert();
        self.view.scroll_to_mark(&mark, 0.1, true, 0.0, 0.1);
        self.view.grab_focus();
    }

    /// Shows the slide at the cursor in the preview.
    fn follow_cursor(&self) {
        if self.loading.get() || !self.preview_visible() || self.preview_kind() != Some(PreviewKind::Slides) {
            return;
        }
        let line = self.buffer.iter_at_mark(&self.buffer.get_insert()).line().max(0) as usize;
        let index = slides::slide_at(&self.slide_lines.borrow(), line);
        if index == self.shown_slide.replace(index) {
            return;
        }
        let preview = self.preview.borrow();
        let Some(p) = preview.as_ref() else { return };
        if p.shell.get() == Shell::Slides && p.loaded.get() {
            let js = format!("QN.showSlide({index})");
            p.webview.evaluate_javascript(&js, None, None, None::<&gio::Cancellable>, |_| {});
        }
    }

    /// Renders the presentation into the preview, showing the slide at the cursor.
    fn render_slides(&self) {
        let deck = slides::parse(&self.text());
        let line = self.buffer.iter_at_mark(&self.buffer.get_insert()).line().max(0) as usize;
        let index = slides::slide_at(&deck.lines, line);
        self.shown_slide.set(index);
        *self.slide_lines.borrow_mut() = deck.lines.clone();
        let js = format!("QN.update({}, {index})", deck.json());
        let preview = self.preview.borrow();
        let Some(p) = preview.as_ref() else { return };
        if p.shell.get() != Shell::Slides {
            let opts = slides::PageOptions { dark: self.shared.theme.get().dark, ..Default::default() };
            self.load_page(p, &slides::page(slides::Mode::Preview, None, opts), Shell::Slides);
            *p.pending.borrow_mut() = Some(js);
        } else if p.loaded.get() {
            p.webview.evaluate_javascript(&js, None, None, None::<&gio::Cancellable>, |_| {});
        } else {
            *p.pending.borrow_mut() = Some(js);
        }
    }

    fn render_kanban(&self) {
        let board = kanban::parse(&self.text());
        let js = format!("QK.update({})", serde_json::to_string(&board).unwrap_or_default());
        let preview = self.preview.borrow();
        let Some(p) = preview.as_ref() else { return };
        if p.shell.get() != Shell::Kanban {
            let theme = self.shared.theme.get();
            self.load_page(p, &kanban::page(theme.dark, &theme.preview_vars()), Shell::Kanban);
            *p.pending.borrow_mut() = Some(js);
        } else if p.loaded.get() {
            p.webview.evaluate_javascript(&js, None, None, None::<&gio::Cancellable>, |_| {});
        } else {
            *p.pending.borrow_mut() = Some(js);
        }
    }

    /// Writes a board edited in the board view back to the source.
    fn apply_board(&self, value: serde_json::Value) {
        let Ok(mut board) = serde_json::from_value::<kanban::Board>(value) else { return };
        kanban::normalize(&mut board);
        let text = kanban::to_markdown(&board);
        if text == self.text() {
            return;
        }
        let (mut start, mut end) = self.buffer.bounds();
        self.buffer.begin_user_action();
        self.buffer.delete(&mut start, &mut end);
        self.buffer.insert(&mut start, &text);
        self.buffer.end_user_action();
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

    /// Shows `html` in the document shell (loading the shell first if needed).
    fn set_preview_html(&self, html: &str) {
        let preview = self.preview.borrow();
        let Some(p) = preview.as_ref() else { return };
        let js = format!("setContent({})", serde_json::to_string(html).unwrap_or_default());
        if p.shell.get() != Shell::Doc {
            self.load_page(p, &html::preview_shell(self.shared.theme.get()), Shell::Doc);
            *p.pending.borrow_mut() = Some(js);
        } else if p.loaded.get() {
            p.webview.evaluate_javascript(&js, None, None, None::<&gio::Cancellable>, |_| {});
        } else {
            *p.pending.borrow_mut() = Some(js);
        }
    }

    /// Shows the buffer as an HTML page, keeping the scroll position.
    fn render_html_page(self: &Rc<Self>) {
        let preview = self.preview.borrow();
        let Some(p) = preview.as_ref() else { return };
        if p.loaded.get() && p.shell.get() == Shell::None {
            let weak = Rc::downgrade(self);
            p.webview.evaluate_javascript(
                "window.scrollY",
                None,
                None,
                None::<&gio::Cancellable>,
                move |result| {
                    let y = result.map(|v| v.to_double()).unwrap_or(0.0);
                    if let Some(t) = weak.upgrade() {
                        t.load_html_document(y);
                    }
                },
            );
        } else {
            drop(preview);
            self.load_html_document(0.0);
        }
    }

    fn load_html_document(&self, scroll_y: f64) {
        let preview = self.preview.borrow();
        let Some(p) = preview.as_ref() else { return };
        self.load_page(p, &self.text(), Shell::None);
        *p.pending.borrow_mut() = (scroll_y > 0.0).then(|| format!("window.scrollTo(0, {scroll_y})"));
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
            Some(PreviewKind::Markdown) => self.set_preview_html(&markdown::to_html_interactive(&self.text())),
            Some(PreviewKind::Rst) => self.set_preview_html(&rst::to_html(&self.text(), self.base_dir().as_deref())),
            Some(PreviewKind::PlantUml) => self.render_plantuml(),
            Some(PreviewKind::Html) => self.render_html_page(),
            Some(PreviewKind::Slides) => self.render_slides(),
            Some(PreviewKind::Kanban) => self.render_kanban(),
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
                Err(e) => t.set_preview_html(&format!("<p class=\"message error\">{}</p>", html::escape(&e))),
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
            split: self.split.get(),
            lock: self.lock(),
        }
    }
}

fn result_markup(result: &LineResult) -> String {
    let esc = |s: &str| glib::markup_escape_text(s).to_string();
    match result {
        LineResult::Empty => String::new(),
        LineResult::Value(v) => format!("<span alpha=\"70%\">= {}</span>", esc(&calc::format_number(*v))),
        LineResult::Text(t) => format!("<b>{}</b>", esc(&truncate(t, 80))),
        LineResult::Error(e) => format!("<span foreground=\"#e5534b\">⚠ {}</span>", esc(&truncate(e, 60))),
    }
}

pub fn preview_bg(theme: &Theme) -> gdk::RGBA {
    gdk::RGBA::parse(theme.background()).unwrap_or(gdk::RGBA::WHITE)
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
