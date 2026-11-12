//! Vertical tab list: a sidebar alternative to `AdwTabBar`, bound to the
//! pages of an `AdwTabView`. Supports selection, closing (button or middle
//! click), reordering by drag and drop and the tab view's context menu.

use adw::prelude::*;
use gtk::{gdk, gio, glib};
use std::rc::Rc;

/// Called with the page a context menu is opened for, and `None` once it closed.
pub type MenuHook = Box<dyn Fn(Option<&adw::TabPage>)>;

pub struct TabList {
    pub root: gtk::ScrolledWindow,
    list: gtk::ListBox,
    view: adw::TabView,
}

impl TabList {
    pub fn new(view: &adw::TabView, on_menu: MenuHook) -> Rc<Self> {
        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::Single);
        list.add_css_class("navigation-sidebar");
        list.add_css_class("tab-sidebar");
        let root = gtk::ScrolledWindow::builder()
            .child(&list)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();
        root.add_css_class("tab-sidebar-scroll");

        let this = Rc::new(Self { root, list, view: view.clone() });
        let on_menu: Rc<MenuHook> = Rc::new(on_menu);

        let weak = Rc::downgrade(&this);
        let menu = on_menu.clone();
        this.list.bind_model(Some(&view.pages()), move |item| {
            let page = item.clone().downcast::<adw::TabPage>().expect("TabView pages are TabPages");
            match weak.upgrade() {
                Some(t) => t.build_row(&page, menu.clone()).upcast(),
                None => gtk::ListBoxRow::new().upcast(),
            }
        });

        let weak = Rc::downgrade(&this);
        this.list.connect_row_selected(move |_, row| {
            let (Some(t), Some(row)) = (weak.upgrade(), row) else { return };
            if row.index() < 0 || row.index() >= t.view.n_pages() {
                return;
            }
            let page = t.view.nth_page(row.index());
            if t.view.selected_page().as_ref() != Some(&page) {
                t.view.set_selected_page(&page);
            }
        });

        // Rows are recreated when pages are added, removed or moved.
        let weak = Rc::downgrade(&this);
        view.pages().connect_items_changed(move |_, _, _, _| {
            let weak = weak.clone();
            glib::idle_add_local_once(move || {
                if let Some(t) = weak.upgrade() {
                    t.sync_selection();
                }
            });
        });
        this
    }

    /// Selects the row of the tab view's selected page.
    pub fn sync_selection(&self) {
        let row = self
            .view
            .selected_page()
            .and_then(|p| self.list.row_at_index(self.view.page_position(&p)));
        if self.list.selected_row() != row {
            self.list.select_row(row.as_ref());
        }
        if let Some(row) = row {
            // Keep the selected tab visible in long lists.
            let list = self.list.clone();
            glib::idle_add_local_once(move || {
                if let Some(adj) = list.adjustment() {
                    if let Some(bounds) = row.compute_bounds(&list) {
                        let (y, h) = (bounds.y() as f64, bounds.height() as f64);
                        if y < adj.value() {
                            adj.set_value(y);
                        } else if y + h > adj.value() + adj.page_size() {
                            adj.set_value(y + h - adj.page_size());
                        }
                    }
                }
            });
        }
    }

    /// Rows with the page each one shows.
    pub fn rows(&self) -> Vec<(gtk::ListBoxRow, adw::TabPage)> {
        (0..self.view.n_pages())
            .filter_map(|i| self.list.row_at_index(i).map(|r| (r, self.view.nth_page(i))))
            .collect()
    }

    fn build_row(self: &Rc<Self>, page: &adw::TabPage, on_menu: Rc<MenuHook>) -> gtk::ListBoxRow {
        let icon = gtk::Image::new();
        page.bind_property("icon", &icon, "gicon").sync_create().build();
        page.bind_property("icon", &icon, "visible")
            .transform_to(|_, icon: Option<gio::Icon>| Some(icon.is_some()))
            .sync_create()
            .build();

        let title = gtk::Label::builder()
            .xalign(0.0)
            .hexpand(true)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        page.bind_property("title", &title, "label").sync_create().build();

        let close = gtk::Button::builder()
            .icon_name("window-close-symbolic")
            .valign(gtk::Align::Center)
            .build();
        close.add_css_class("flat");
        close.add_css_class("circular");
        close.add_css_class("tab-close");

        let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        content.append(&icon);
        content.append(&title);
        content.append(&close);

        let row = gtk::ListBoxRow::builder().child(&content).focusable(false).build();
        page.bind_property("tooltip", &row, "tooltip-markup").sync_create().build();

        let view = self.view.downgrade();
        let pg = page.downgrade();
        close.connect_clicked(move |_| {
            if let (Some(view), Some(page)) = (view.upgrade(), pg.upgrade()) {
                view.close_page(&page);
            }
        });

        // Middle click closes, right click opens the tab menu.
        let click = gtk::GestureClick::new();
        click.set_button(0);
        let view = self.view.downgrade();
        let pg = page.downgrade();
        let row_ref = row.downgrade();
        click.connect_pressed(move |gesture, _, x, y| {
            let (Some(view), Some(page), Some(row)) = (view.upgrade(), pg.upgrade(), row_ref.upgrade()) else {
                return;
            };
            match gesture.current_button() {
                gdk::BUTTON_MIDDLE => {
                    gesture.set_state(gtk::EventSequenceState::Claimed);
                    view.close_page(&page);
                }
                gdk::BUTTON_SECONDARY => {
                    gesture.set_state(gtk::EventSequenceState::Claimed);
                    let Some(model) = view.menu_model() else { return };
                    on_menu(Some(&page));
                    let popover = gtk::PopoverMenu::from_model_full(&model, gtk::PopoverMenuFlags::NESTED);
                    popover.set_parent(&row);
                    popover.set_has_arrow(false);
                    popover.set_halign(gtk::Align::Start);
                    popover.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
                    let on_menu = on_menu.clone();
                    popover.connect_closed(move |p| {
                        on_menu(None);
                        let p = p.clone();
                        glib::idle_add_local_once(move || p.unparent());
                    });
                    popover.popup();
                }
                _ => {}
            }
        });
        row.add_controller(click);

        // Reordering: drag a row onto another one.
        let drag = gtk::DragSource::new();
        drag.set_actions(gdk::DragAction::MOVE);
        let view = self.view.downgrade();
        let pg = page.downgrade();
        drag.connect_prepare(move |_, _, _| {
            let (view, page) = (view.upgrade()?, pg.upgrade()?);
            let position = view.page_position(&page) as u32;
            Some(gdk::ContentProvider::for_value(&position.to_value()))
        });
        let row_ref = row.downgrade();
        drag.connect_drag_begin(move |source, _| {
            if let Some(row) = row_ref.upgrade() {
                source.set_icon(Some(&gtk::WidgetPaintable::new(Some(&row))), 0, 0);
            }
        });
        row.add_controller(drag);

        let drop = gtk::DropTarget::new(u32::static_type(), gdk::DragAction::MOVE);
        let view = self.view.downgrade();
        let pg = page.downgrade();
        drop.connect_drop(move |_, value, _, _| {
            let (Some(view), Some(target), Ok(from)) = (view.upgrade(), pg.upgrade(), value.get::<u32>()) else {
                return false;
            };
            if from as i32 >= view.n_pages() {
                return false;
            }
            let moved = view.nth_page(from as i32);
            let to = view.page_position(&target);
            if moved != target {
                view.reorder_page(&moved, to);
                view.set_selected_page(&moved);
            }
            true
        });
        row.add_controller(drop);

        row
    }
}
