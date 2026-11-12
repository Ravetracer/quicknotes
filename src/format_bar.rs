//! Formatting toolbar shown above Markdown editors. Its buttons trigger the
//! `win.md-*` actions (see [`crate::md_format`]).

use crate::icons;
use crate::md_format::COLORS;
use gtk::prelude::*;
use gtk::{gdk, gio};
use std::cell::Cell;

/// Action name of a text colour, e.g. `md-color-red`.
pub fn color_action(label: &str) -> String {
    format!("md-color-{}", label.to_lowercase())
}

pub fn build() -> gtk::Box {
    ensure_css();
    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    bar.add_css_class("format-bar");
    bar.set_visible(false);

    let headings = gio::Menu::new();
    for level in 1..=6 {
        headings.append(Some(&t!("Heading {}", level)), Some(&format!("win.md-h{level}")));
    }
    let normal = gio::Menu::new();
    normal.append(Some(t!("Normal Text")), Some("win.md-paragraph"));
    headings.append_section(None, &normal);
    let heading = gtk::MenuButton::builder()
        .icon_name(icons::format_icon("heading"))
        .menu_model(&headings)
        .tooltip_text(t!("Heading"))
        .build();
    heading.add_css_class("flat");

    let groups: [Vec<gtk::Widget>; 4] = [
        vec![
            heading.upcast(),
            icon("bold", t!("Bold (Ctrl+B)"), "win.md-bold"),
            icon("italic", t!("Italic (Ctrl+I)"), "win.md-italic"),
            icon("strikethrough", t!("Strikethrough"), "win.md-strike"),
            icon("code", t!("Inline Code"), "win.md-code"),
            color_button(),
        ],
        vec![
            icon("bullet-list", t!("Bulleted List"), "win.md-bullet"),
            icon("numbered-list", t!("Numbered List"), "win.md-numbered"),
            icon("checklist", t!("Checklist"), "win.md-task"),
        ],
        vec![
            icon("quote", t!("Quote"), "win.md-quote"),
            icon("code-block", t!("Code Block"), "win.md-code-block"),
            icon("rule", t!("Horizontal Rule"), "win.md-rule"),
            icon("table", t!("Insert Table"), "win.table-insert"),
        ],
        vec![
            icon("link", t!("Link (Ctrl+K)"), "win.md-link"),
            icon("image", t!("Image"), "win.md-image"),
        ],
    ];
    for (i, group) in groups.iter().enumerate() {
        if i > 0 {
            bar.append(&gtk::Separator::new(gtk::Orientation::Vertical));
        }
        for widget in group {
            bar.append(widget);
        }
    }
    bar
}

fn icon(name: &str, tip: &str, action: &str) -> gtk::Widget {
    let b = gtk::Button::builder().icon_name(icons::format_icon(name)).tooltip_text(tip).action_name(action).build();
    b.add_css_class("flat");
    b.upcast()
}

/// Colour picker: a grid of swatches and *Remove Colour*.
fn color_button() -> gtk::Widget {
    let grid = gtk::Grid::builder().row_spacing(4).column_spacing(4).build();
    let popover = gtk::Popover::new();
    for (i, (label, _)) in COLORS.iter().enumerate() {
        let swatch = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        swatch.add_css_class("color-swatch");
        swatch.add_css_class(&format!("swatch-{i}"));
        let b = gtk::Button::builder()
            .child(&swatch)
            .tooltip_text(crate::i18n::tr(label))
            .action_name(format!("win.{}", color_action(label)))
            .build();
        b.add_css_class("flat");
        let p = popover.downgrade();
        b.connect_clicked(move |_| {
            if let Some(p) = p.upgrade() {
                p.popdown();
            }
        });
        grid.attach(&b, (i % 5) as i32, (i / 5) as i32, 1, 1);
    }
    let remove = gtk::Button::builder().label(t!("Remove Colour")).action_name("win.md-color-none").build();
    remove.add_css_class("flat");
    let p = popover.downgrade();
    remove.connect_clicked(move |_| {
        if let Some(p) = p.upgrade() {
            p.popdown();
        }
    });
    let content = gtk::Box::new(gtk::Orientation::Vertical, 6);
    content.append(&grid);
    content.append(&remove);
    popover.set_child(Some(&content));
    let button = gtk::MenuButton::builder()
        .icon_name(icons::format_icon("color"))
        .popover(&popover)
        .tooltip_text(t!("Text color"))
        .build();
    button.add_css_class("flat");
    button.upcast()
}

/// Swatch colours, registered once per process.
fn ensure_css() {
    thread_local!(static DONE: Cell<bool> = const { Cell::new(false) });
    if DONE.with(|d| d.replace(true)) {
        return;
    }
    let Some(display) = gdk::Display::default() else { return };
    let mut css = String::new();
    for (i, (_, color)) in COLORS.iter().enumerate() {
        css.push_str(&format!(".color-swatch.swatch-{i} {{ background-color: {color}; }}\n"));
    }
    let provider = gtk::CssProvider::new();
    provider.load_from_string(&css);
    gtk::style_context_add_provider_for_display(&display, &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
}
