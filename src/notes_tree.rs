//! The notes sidebar: the folder tree of the notes library (see
//! [`crate::library`]) with drag and drop and context menus. Menu entries
//! use the window's `lib-*` actions; everything else is reported as an
//! [`Event`].

use crate::library::{Row, TRASH, UNGROUPED};
use adw::prelude::*;
use gtk::{gdk, gio, glib};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Open (or switch to) a note.
    Open(String),
    /// Expand or collapse a folder.
    Toggle(String),
    /// Move a note (`item`) or folder into `target` (a folder id, or
    /// [`UNGROUPED`] / [`TRASH`]).
    Move { folder: bool, id: String, target: String },
}

pub struct NotesTree {
    pub root: gtk::Box,
    list: gtk::ListBox,
    rows: RefCell<Vec<Row>>,
    current: RefCell<Option<String>>,
    on_event: Box<dyn Fn(Event)>,
}

/// Drag data: `folder:<id>` or `item:<id>`.
fn drag_value(row: &Row) -> String {
    match row {
        Row::Folder { id, .. } => format!("folder:{id}"),
        Row::Item { item, .. } => format!("item:{}", item.id),
    }
}

/// Where something dropped on `row` goes.
fn drop_target(row: &Row) -> String {
    match row {
        Row::Folder { id, .. } => id.clone(),
        Row::Item { item, .. } if item.trashed => TRASH.into(),
        Row::Item { item, .. } => item.folder.clone().unwrap_or_else(|| UNGROUPED.into()),
    }
}

impl NotesTree {
    pub fn new(on_event: impl Fn(Event) + 'static) -> Rc<Self> {
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        header.add_css_class("notes-tree-header");
        let title = gtk::Label::builder().label(t!("Notes")).xalign(0.0).hexpand(true).build();
        title.add_css_class("heading");
        header.append(&title);
        for (icon, tip, action) in [
            ("folder-new-symbolic", t!("New Folder"), "win.lib-new-folder::"),
            ("document-new-symbolic", t!("New Note"), "win.new-note"),
        ] {
            let b = gtk::Button::builder().icon_name(icon).tooltip_text(tip).build();
            b.set_detailed_action_name(action);
            b.add_css_class("flat");
            header.append(&b);
        }
        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::Single);
        list.add_css_class("navigation-sidebar");
        list.add_css_class("notes-tree");
        let scroll = gtk::ScrolledWindow::builder()
            .child(&list)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.add_css_class("notes-sidebar");
        root.append(&header);
        root.append(&scroll);
        ensure_css();

        let this = Rc::new(Self { root, list, rows: RefCell::new(Vec::new()), current: RefCell::new(None), on_event: Box::new(on_event) });
        let weak = Rc::downgrade(&this);
        this.list.connect_row_activated(move |_, row| {
            let Some(this) = weak.upgrade() else { return };
            let Some(r) = this.rows.borrow().get(row.index() as usize).cloned() else { return };
            match r {
                Row::Folder { id, .. } => (this.on_event)(Event::Toggle(id)),
                Row::Item { item, .. } => (this.on_event)(Event::Open(item.id)),
            }
        });
        this
    }

    /// Shows `rows`, with the row of note `current` selected. Does nothing
    /// if nothing changed.
    pub fn set_rows(self: &Rc<Self>, rows: Vec<Row>, current: Option<&str>) {
        let current = current.map(str::to_owned);
        if *self.rows.borrow() == rows && *self.current.borrow() == current {
            return;
        }
        let rebuild = *self.rows.borrow() != rows;
        *self.rows.borrow_mut() = rows;
        *self.current.borrow_mut() = current.clone();
        if rebuild {
            while let Some(child) = self.list.first_child() {
                self.list.remove(&child);
            }
            let rows = self.rows.borrow().clone();
            for row in &rows {
                let widget = self.build_row(row);
                self.list.append(&widget);
            }
        }
        let index = self.rows.borrow().iter().position(|r| matches!(r, Row::Item { item, .. } if Some(&item.id) == current.as_ref()));
        match index.and_then(|i| self.list.row_at_index(i as i32)) {
            Some(row) => self.list.select_row(Some(&row)),
            None => self.list.unselect_all(),
        }
    }

    fn build_row(self: &Rc<Self>, row: &Row) -> gtk::ListBoxRow {
        let line = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let label = gtk::Label::builder().xalign(0.0).hexpand(true).ellipsize(gtk::pango::EllipsizeMode::End).build();
        let (depth, tooltip) = match row {
            Row::Folder { id, name, depth, collapsed, count } => {
                let arrow = gtk::Image::from_icon_name(if *collapsed { "pan-end-symbolic" } else { "pan-down-symbolic" });
                arrow.add_css_class("dim-label");
                line.append(&arrow);
                let icon = match id.as_str() {
                    TRASH => "user-trash-symbolic",
                    UNGROUPED => "view-list-symbolic",
                    _ => "folder-symbolic",
                };
                line.append(&gtk::Image::from_icon_name(icon));
                label.set_text(name);
                label.add_css_class("notes-folder");
                line.append(&label);
                let n = gtk::Label::new(Some(&count.to_string()));
                n.add_css_class("dim-label");
                n.add_css_class("caption");
                line.append(&n);
                (*depth, None)
            }
            Row::Item { item, depth } => {
                // Room of the folder arrow.
                let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                spacer.set_size_request(16, -1);
                line.append(&spacer);
                line.append(&gtk::Image::from_icon_name(item.icon));
                label.set_text(&item.title);
                if item.open {
                    label.add_css_class("notes-open");
                }
                if item.missing || item.trashed {
                    label.add_css_class("dim-label");
                }
                line.append(&label);
                if item.missing {
                    let warn = gtk::Image::from_icon_name("dialog-warning-symbolic");
                    warn.set_tooltip_text(Some(t!("The file no longer exists")));
                    line.append(&warn);
                }
                (*depth, item.file.then(|| t!("File")))
            }
        };
        line.set_margin_start(6 + depth as i32 * 14);
        line.set_margin_end(6);
        let widget = gtk::ListBoxRow::builder().child(&line).build();
        if let Some(t) = tooltip {
            widget.set_tooltip_text(Some(t));
        }

        // Drag and drop (not for the special places).
        let movable = !matches!(row, Row::Folder { id, .. } if id.starts_with(':'));
        if movable {
            let source = gtk::DragSource::new();
            source.set_actions(gdk::DragAction::MOVE);
            let value = drag_value(row);
            source.connect_prepare(move |_, _, _| Some(gdk::ContentProvider::for_value(&value.to_value())));
            widget.add_controller(source);
        }
        let target = gtk::DropTarget::new(glib::Type::STRING, gdk::DragAction::MOVE);
        let dest = drop_target(row);
        let weak = Rc::downgrade(self);
        target.connect_drop(move |_, value, _, _| {
            let (Some(this), Ok(data)) = (weak.upgrade(), value.get::<String>()) else { return false };
            let Some((kind, id)) = data.split_once(':') else { return false };
            (this.on_event)(Event::Move { folder: kind == "folder", id: id.to_owned(), target: dest.clone() });
            true
        });
        let w = widget.downgrade();
        target.connect_enter(move |_, _, _| {
            if let Some(w) = w.upgrade() {
                w.add_css_class("notes-drop");
            }
            gdk::DragAction::MOVE
        });
        let w = widget.downgrade();
        target.connect_leave(move |_| {
            if let Some(w) = w.upgrade() {
                w.remove_css_class("notes-drop");
            }
        });
        widget.add_controller(target);

        // Context menu.
        let click = gtk::GestureClick::new();
        click.set_button(gdk::BUTTON_SECONDARY);
        let weak = Rc::downgrade(self);
        let row_data = row.clone();
        let w = widget.downgrade();
        click.connect_pressed(move |gesture, _, x, y| {
            let (Some(this), Some(w)) = (weak.upgrade(), w.upgrade()) else { return };
            gesture.set_state(gtk::EventSequenceState::Claimed);
            this.popup_menu(&row_data, &w, x, y);
        });
        widget.add_controller(click);
        widget
    }

    fn popup_menu(&self, row: &Row, widget: &gtk::ListBoxRow, x: f64, y: f64) {
        let menu = gio::Menu::new();
        let item = |label: &str, action: &str, id: &str| {
            let i = gio::MenuItem::new(Some(label), None);
            i.set_action_and_target_value(Some(action), Some(&id.to_variant()));
            i
        };
        match row {
            Row::Folder { id, .. } if id == TRASH => {
                menu.append_item(&item(t!("Empty Trash"), "win.lib-empty-trash", ""));
            }
            Row::Folder { id, .. } if id == UNGROUPED => {
                menu.append_item(&item(t!("New Note"), "win.lib-new-note", ""));
                menu.append_item(&item(t!("New Folder"), "win.lib-new-folder", ""));
            }
            Row::Folder { id, .. } => {
                let section = gio::Menu::new();
                section.append_item(&item(t!("New Note"), "win.lib-new-note", id));
                section.append_item(&item(t!("New Folder"), "win.lib-new-folder", id));
                menu.append_section(None, &section);
                let section = gio::Menu::new();
                section.append_item(&item(t!("Rename Folder…"), "win.lib-rename-folder", id));
                section.append_item(&item(t!("Delete Folder"), "win.lib-delete-folder", id));
                menu.append_section(None, &section);
            }
            Row::Item { item: it, .. } if it.trashed => {
                menu.append_item(&item(t!("Restore"), "win.lib-restore", &it.id));
                menu.append_item(&item(t!("Delete Permanently"), "win.lib-delete", &it.id));
            }
            Row::Item { item: it, .. } => {
                menu.append_item(&item(t!("Open"), "win.lib-open", &it.id));
                if it.folder.is_some() {
                    menu.append_item(&item(t!("Move to Ungrouped"), "win.lib-ungroup", &it.id));
                }
                if it.file {
                    menu.append_item(&item(t!("Remove from Notes"), "win.lib-remove", &it.id));
                } else {
                    menu.append_item(&item(t!("Move to Trash"), "win.lib-trash", &it.id));
                }
            }
        }
        let popover = gtk::PopoverMenu::from_model(Some(&menu));
        popover.set_has_arrow(false);
        popover.set_halign(gtk::Align::Start);
        // Attached to the sidebar box, which sizes popovers properly.
        let point = widget
            .compute_point(&self.root, &gtk::graphene::Point::new(x as f32, y as f32))
            .unwrap_or_else(|| gtk::graphene::Point::new(x as f32, y as f32));
        popover.set_parent(&self.root);
        popover.set_pointing_to(Some(&gdk::Rectangle::new(point.x() as i32, point.y() as i32, 1, 1)));
        popover.connect_closed(|p| {
            let p = p.clone();
            glib::idle_add_local_once(move || p.unparent());
        });
        popover.popup();
    }
}

fn ensure_css() {
    thread_local!(static DONE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) });
    if DONE.with(|d| d.replace(true)) {
        return;
    }
    let Some(display) = gdk::Display::default() else { return };
    let provider = gtk::CssProvider::new();
    provider.load_from_string(
        ".notes-tree-header { padding: 6px 6px 6px 12px; }
         .notes-tree row { min-height: 28px; padding-top: 0; padding-bottom: 0; }
         .notes-tree .notes-open { font-weight: bold; }
         .notes-tree .notes-folder { font-weight: 600; }
         .notes-tree row.notes-drop { background-color: alpha(@accent_bg_color, 0.25); }",
    );
    gtk::style_context_add_provider_for_display(&display, &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
}
