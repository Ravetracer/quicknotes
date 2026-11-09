//! Preferences dialog: editor font, language, colour themes, tab layout and
//! colours, and backup/restore of the notes.

use crate::store::{self, Settings};
use crate::themes::{self, Theme};
use adw::prelude::*;
use gtk::{gdk, glib};
use std::collections::HashMap;
use std::rc::Rc;

pub const MIN_FONT: f64 = 6.0;
pub const MAX_FONT: f64 = 48.0;

/// Receives the changes made in the dialog.
pub trait Handler {
    /// Applied right away, so the window behind the dialog shows the theme.
    fn set_theme(&self, id: &str);
    fn set_vertical_tabs(&self, vertical: bool);
    fn set_font_family(&self, family: &str);
    fn set_font_size(&self, size: f64);
    fn set_ligatures(&self, on: bool);
    fn set_language(&self, language: &str);
    /// `None` restores the default colour.
    fn set_tab_color(&self, kind: &str, color: Option<String>);
    fn backup(&self);
    fn restore_backup(&self);
    fn open_data_dir(&self);
}

/// A tab type with a configurable colour.
pub struct TabColor {
    pub key: &'static str,
    pub label: &'static str,
    pub default: &'static str,
}

const LANGUAGES: [&str; 3] = ["system", "en", "de"];

pub fn show(parent: &impl IsA<gtk::Widget>, settings: &Settings, colors: Vec<TabColor>, handler: Rc<dyn Handler>) {
    let dialog = adw::PreferencesDialog::new();
    dialog.set_title(t!("Preferences"));
    dialog.set_search_enabled(true);
    // Room for four page titles; low enough to keep the window visible behind it.
    dialog.set_content_width(760);
    dialog.set_content_height(620);
    dialog.add(&general_page(parent, settings, &handler));
    dialog.add(&themes_page(settings, &handler));
    dialog.add(&tabs_page(settings, colors, &handler));
    dialog.add(&import_export_page(&dialog, &handler));
    dialog.present(Some(parent));
}

fn general_page(parent: &impl IsA<gtk::Widget>, settings: &Settings, handler: &Rc<dyn Handler>) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::builder()
        .title(t!("General"))
        .icon_name("preferences-system-symbolic")
        .build();

    // ----- editor font ---------------------------------------------------
    let font = adw::PreferencesGroup::builder()
        .title(t!("Editor Font"))
        .description(t!("The size can also be changed with Ctrl+mouse wheel or Ctrl +/−/0."))
        .build();
    let family_row = adw::ActionRow::builder().title(t!("Font")).build();
    let family = build_font_dropdown(parent, &settings.font_family);
    family.set_valign(gtk::Align::Center);
    select_string(&family, &settings.font_family);
    let h = handler.clone();
    family.connect_selected_item_notify(move |drop| {
        if let Some(name) = drop.selected_item().and_downcast::<gtk::StringObject>() {
            h.set_font_family(&name.string());
        }
    });
    family_row.add_suffix(&family);
    family_row.set_activatable_widget(Some(&family));
    font.add(&family_row);

    let size = adw::SpinRow::with_range(MIN_FONT, MAX_FONT, 1.0);
    size.set_title(t!("Size (pt)"));
    size.set_value(settings.font_size);
    let h = handler.clone();
    size.connect_value_notify(move |row| h.set_font_size(row.value()));
    font.add(&size);

    let ligatures = adw::SwitchRow::builder()
        .title(t!("Font Ligatures"))
        .subtitle(t!("Combines sequences like != or => in fonts such as Fira Code"))
        .active(settings.ligatures)
        .build();
    let h = handler.clone();
    ligatures.connect_active_notify(move |row| h.set_ligatures(row.is_active()));
    font.add(&ligatures);
    page.add(&font);

    // ----- language ------------------------------------------------------
    let language = adw::PreferencesGroup::builder().title(t!("Language")).build();
    let lang_row = adw::ComboRow::builder()
        .title(t!("User Interface"))
        .subtitle(t!("Applies after a restart"))
        .model(&gtk::StringList::new(&[t!("System Default"), "English", "Deutsch"]))
        .selected(LANGUAGES.iter().position(|l| *l == settings.language).unwrap_or(0) as u32)
        .build();
    let h = handler.clone();
    lang_row.connect_selected_notify(move |row| {
        if let Some(lang) = LANGUAGES.get(row.selected() as usize) {
            h.set_language(lang);
        }
    });
    language.add(&lang_row);
    page.add(&language);
    page
}

fn themes_page(settings: &Settings, handler: &Rc<dyn Handler>) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::builder()
        .title(t!("Themes"))
        .icon_name("applications-graphics-symbolic")
        .description(t!("The theme is applied right away. Search themes by name with Ctrl+F."))
        .build();
    let current = themes::find(&settings.theme, settings.dark).id.clone();
    // Check marks of all rows, to move the mark on selection.
    let marks: Rc<std::cell::RefCell<Vec<(String, gtk::Image)>>> = Rc::default();

    for (key, name, _) in themes::SOURCES {
        let title = if key == "quicknotes" { t!("Default") } else { name };
        let group = adw::PreferencesGroup::builder().title(title).build();
        for theme in themes::all().iter().filter(|t| t.source == key) {
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(theme.label()).as_str())
                .subtitle(if theme.dark { t!("Dark") } else { t!("Light") })
                .activatable(true)
                .build();
            row.add_prefix(&swatch(theme));
            let mark = gtk::Image::from_icon_name("object-select-symbolic");
            mark.set_visible(theme.id == current);
            row.add_suffix(&mark);
            marks.borrow_mut().push((theme.id.clone(), mark));

            let (h, marks, id) = (handler.clone(), marks.clone(), theme.id.clone());
            row.connect_activated(move |_| {
                for (other, mark) in marks.borrow().iter() {
                    mark.set_visible(*other == id);
                }
                h.set_theme(&id);
            });
            group.add(&row);
        }
        page.add(&group);
    }
    page
}

/// Small preview of a theme: background with a few lines of "code".
fn swatch(theme: &Theme) -> gtk::DrawingArea {
    let colors: Vec<gdk::RGBA> = theme
        .swatch()
        .iter()
        .map(|c| gdk::RGBA::parse(*c).unwrap_or(gdk::RGBA::BLACK))
        .collect();
    let area = gtk::DrawingArea::builder()
        .content_width(64)
        .content_height(40)
        .valign(gtk::Align::Center)
        .margin_top(6)
        .margin_bottom(6)
        .build();
    area.set_draw_func(move |_, cr, w, h| {
        let (w, h) = (w as f64, h as f64);
        let set = |c: &gdk::RGBA| cr.set_source_rgba(c.red() as f64, c.green() as f64, c.blue() as f64, 1.0);
        rounded(cr, 0.5, 0.5, w - 1.0, h - 1.0, 6.0);
        set(&colors[0]);
        let _ = cr.fill_preserve();
        set(&colors[1]);
        cr.set_line_width(1.0);
        let _ = cr.stroke();
        // (indent, length, colour) per bar, three lines.
        let lines: [&[(f64, f64, usize)]; 3] =
            [&[(0.0, 0.22, 2), (0.27, 0.34, 3)], &[(0.12, 0.5, 4)], &[(0.0, 0.62, 5)]];
        cr.set_line_width(3.0);
        cr.set_line_cap(gtk::cairo::LineCap::Round);
        for (i, bars) in lines.iter().enumerate() {
            let y = h * (0.3 + 0.2 * i as f64);
            for (start, len, color) in bars.iter() {
                set(&colors[*color]);
                cr.move_to(8.0 + start * (w - 16.0), y);
                cr.line_to(8.0 + (start + len) * (w - 16.0), y);
                let _ = cr.stroke();
            }
        }
    });
    area
}

fn rounded(cr: &gtk::cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    use std::f64::consts::{FRAC_PI_2, PI};
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -FRAC_PI_2, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, FRAC_PI_2);
    cr.arc(x + r, y + h - r, r, FRAC_PI_2, PI);
    cr.arc(x + r, y + r, r, PI, 3.0 * FRAC_PI_2);
    cr.close_path();
}

fn tabs_page(settings: &Settings, colors: Vec<TabColor>, handler: &Rc<dyn Handler>) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::builder()
        .title(t!("Tabs"))
        .icon_name("view-paged-symbolic")
        .build();

    let layout = adw::PreferencesGroup::builder().title(t!("Layout")).build();
    let position = adw::ComboRow::builder()
        .title(t!("Tab Position"))
        .model(&gtk::StringList::new(&[t!("Horizontal (top)"), t!("Vertical (side)")]))
        .selected(settings.vertical_tabs as u32)
        .build();
    let h = handler.clone();
    position.connect_selected_notify(move |row| h.set_vertical_tabs(row.selected() == 1));
    layout.add(&position);
    page.add(&layout);

    let group = adw::PreferencesGroup::builder()
        .title(t!("Tab Colours"))
        .description(t!("Tabs are coloured by their type; plain notes keep the default look."))
        .build();
    for color in colors {
        let row = adw::ActionRow::builder().title(color.label).build();
        let default = parse_color(color.default);
        let current = settings.tab_colors.get(color.key).and_then(|c| gdk::RGBA::parse(c.as_str()).ok());

        let button = gtk::ColorDialogButton::new(Some(gtk::ColorDialog::builder().with_alpha(false).build()));
        button.set_valign(gtk::Align::Center);
        button.set_rgba(&current.unwrap_or(default));

        let reset = gtk::Button::builder()
            .icon_name("edit-undo-symbolic")
            .tooltip_text(t!("Restore default colour"))
            .valign(gtk::Align::Center)
            .sensitive(current.is_some_and(|c| to_hex(&c) != to_hex(&default)))
            .build();
        reset.add_css_class("flat");

        let h = handler.clone();
        let key = color.key;
        let reset_ref = reset.downgrade();
        button.connect_rgba_notify(move |b| {
            let hex = to_hex(&b.rgba());
            let is_default = hex == to_hex(&default);
            h.set_tab_color(key, (!is_default).then_some(hex));
            if let Some(r) = reset_ref.upgrade() {
                r.set_sensitive(!is_default);
            }
        });
        let button_ref = button.downgrade();
        reset.connect_clicked(move |_| {
            if let Some(b) = button_ref.upgrade() {
                b.set_rgba(&default);
            }
        });

        row.add_suffix(&reset);
        row.add_suffix(&button);
        row.set_activatable_widget(Some(&button));
        group.add(&row);
    }
    page.add(&group);
    page
}

fn import_export_page(dialog: &adw::PreferencesDialog, handler: &Rc<dyn Handler>) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::builder()
        .title(t!("Import/Export"))
        .icon_name("document-save-symbolic")
        .build();

    let backup = adw::PreferencesGroup::builder()
        .title(t!("Backup"))
        .description(t!("All tabs, notes and settings in one .tar.gz file."))
        .build();
    let export = button_row(t!("Back Up Notes…"), t!("Export all notes into a backup file"), "document-save-as-symbolic");
    let h = handler.clone();
    export.connect_activated(move |_| h.backup());
    backup.add(&export);

    let import = button_row(
        t!("Restore Backup…"),
        t!("Add the tabs of a backup or replace the current ones"),
        "document-open-symbolic",
    );
    let h = handler.clone();
    let dlg = dialog.downgrade();
    import.connect_activated(move |_| {
        // A restore may replace the settings shown here.
        if let Some(d) = dlg.upgrade() {
            d.close();
        }
        h.restore_backup();
    });
    backup.add(&import);
    page.add(&backup);

    let storage = adw::PreferencesGroup::builder().title(t!("Storage")).build();
    let dir = store::data_dir();
    let home = dirs::home_dir().unwrap_or_default();
    let shown = match dir.strip_prefix(&home) {
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => dir.display().to_string(),
    };
    let folder = button_row(t!("Open Notes Folder"), &shown, "folder-open-symbolic");
    let h = handler.clone();
    folder.connect_activated(move |_| h.open_data_dir());
    storage.add(&folder);
    page.add(&storage);
    page
}

fn button_row(title: &str, subtitle: &str, icon: &str) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(title)
        .subtitle(glib::markup_escape_text(subtitle).as_str())
        .activatable(true)
        .build();
    row.add_suffix(&gtk::Image::from_icon_name(icon));
    row
}

fn parse_color(hex: &str) -> gdk::RGBA {
    gdk::RGBA::parse(hex).unwrap_or(gdk::RGBA::BLACK)
}

pub fn to_hex(c: &gdk::RGBA) -> String {
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", byte(c.red()), byte(c.green()), byte(c.blue()))
}

/// Selects the entry `name` (case-insensitive) of a string drop-down.
fn select_string(drop: &gtk::DropDown, name: &str) {
    let Some(model) = drop.model() else { return };
    let position = (0..model.n_items()).find(|&i| {
        model
            .item(i)
            .and_downcast::<gtk::StringObject>()
            .is_some_and(|s| s.string().eq_ignore_ascii_case(name))
    });
    if let Some(position) = position {
        drop.set_selected(position);
    }
}

/// Font families for the editor font selector: the system default
/// "Monospace", other monospace fonts, then all remaining fonts.
/// Returns the names and how many belong to the monospace group.
fn font_families(widget: &impl IsA<gtk::Widget>, current: &str) -> (Vec<String>, usize) {
    let (mut mono, mut other): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
    for family in widget.pango_context().list_families() {
        let name = family.name().to_string();
        if family.is_monospace() {
            mono.push(name);
        } else {
            other.push(name);
        }
    }
    for list in [&mut mono, &mut other] {
        list.sort_by_key(|n| n.to_lowercase());
        list.dedup();
        list.retain(|n| !n.eq_ignore_ascii_case("Monospace"));
    }
    let mut names = vec!["Monospace".to_owned()];
    names.extend(mono);
    // A configured font that is not installed (anymore) stays selectable.
    let known = names.iter().chain(&other).any(|n| n.eq_ignore_ascii_case(current));
    if !known && !current.is_empty() {
        names.insert(1, current.to_owned());
    }
    let mono_count = names.len();
    names.extend(other);
    (names, mono_count)
}

/// Searchable font selector. Unlike GtkFontDialog (GTK 4.14), it shows the
/// current font as selected, so Enter keeps it. Each name is rendered in
/// its own font; monospace fonts come first. Group captions live inside the
/// rows because GtkDropDown drops list sections when search is enabled.
fn build_font_dropdown(widget: &impl IsA<gtk::Widget>, current: &str) -> gtk::DropDown {
    let (names, mono_count) = font_families(widget, current);
    let captions: Rc<HashMap<String, &'static str>> = Rc::new(
        [
            names.first().map(|n| (n.clone(), t!("Monospace fonts"))),
            names.get(mono_count).map(|n| (n.clone(), t!("Other fonts"))),
        ]
        .into_iter()
        .flatten()
        .collect(),
    );
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let list = gtk::StringList::new(&refs);

    // Rows: optional group caption above the font name.
    let factory = |in_list: bool| {
        let f = gtk::SignalListItemFactory::new();
        f.connect_setup(move |_, item| {
            let Some(item) = item.downcast_ref::<gtk::ListItem>() else { return };
            let row = gtk::Box::new(gtk::Orientation::Vertical, 2);
            let caption = gtk::Label::builder().xalign(0.0).visible(false).build();
            caption.add_css_class("caption-heading");
            caption.add_css_class("dim-label");
            let name = gtk::Label::builder().xalign(0.0).build();
            if !in_list {
                name.set_ellipsize(gtk::pango::EllipsizeMode::End);
                name.set_max_width_chars(20);
            }
            row.append(&caption);
            row.append(&name);
            item.set_child(Some(&row));
        });
        let captions = captions.clone();
        f.connect_bind(move |_, item| {
            let Some(item) = item.downcast_ref::<gtk::ListItem>() else { return };
            let Some(row) = item.child().and_downcast::<gtk::Box>() else { return };
            let Some(family) = item.item().and_downcast::<gtk::StringObject>().map(|s| s.string()) else {
                return;
            };
            let (Some(caption), Some(name)) = (
                row.first_child().and_downcast::<gtk::Label>(),
                row.last_child().and_downcast::<gtk::Label>(),
            ) else {
                return;
            };
            let group = if in_list { captions.get(family.as_str()).copied() } else { None };
            caption.set_text(group.unwrap_or_default());
            caption.set_visible(group.is_some());
            if group == Some(t!("Other fonts")) {
                row.add_css_class("font-group-start");
            } else {
                row.remove_css_class("font-group-start");
            }
            let mut desc = gtk::pango::FontDescription::new();
            desc.set_family(&family);
            let attrs = gtk::pango::AttrList::new();
            attrs.insert(gtk::pango::AttrFontDesc::new(&desc));
            name.set_attributes(Some(&attrs));
            name.set_text(&family);
        });
        f
    };

    let drop = gtk::DropDown::new(
        Some(list),
        Some(gtk::PropertyExpression::new(
            gtk::StringObject::static_type(),
            None::<&gtk::Expression>,
            "string",
        )),
    );
    drop.set_factory(Some(&factory(false)));
    drop.set_list_factory(Some(&factory(true)));
    drop.set_enable_search(true);
    drop.set_search_match_mode(gtk::StringFilterMatchMode::Substring);
    drop.set_tooltip_text(Some(t!("Editor font")));
    crate::window::enter_picks_first_match(&drop);
    drop
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_colours() {
        assert_eq!(to_hex(&gdk::RGBA::new(1.0, 0.0, 0.5, 1.0)), "#ff0080");
    }
}
