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
    fn set_emmet_tab(&self, on: bool);
    /// Switches a style rule of the YAML check (see `yaml_lint::RULES`).
    fn set_yaml_rule(&self, key: &str, on: bool);
    fn set_feature(&self, feature: crate::features::Feature, on: bool);
    fn set_language(&self, language: &str);
    /// `None` restores the default colour.
    fn set_tab_color(&self, kind: &str, color: Option<String>);
    fn backup(&self);
    fn restore_backup(&self);
    fn open_data_dir(&self);
    /// Sets, changes or removes the app password; `done` gets whether it is on afterwards.
    fn app_password(&self, action: PasswordAction, done: Box<dyn Fn(bool)>);
    fn lock_app(&self);
    /// Signs in to GitHub (device code in the browser, or a pasted token
    /// when `token`); `done` gets the account or an error.
    fn github_sign_in(&self, token: bool, done: Box<dyn Fn(Result<String, String>)>);
    fn github_sign_out(&self);
    /// Sets up the repository for syncing (asks for the sync password);
    /// `done` gets a description of it or an error.
    fn github_connect(&self, repo: String, done: Box<dyn Fn(Result<String, String>)>);
    fn github_disconnect(&self);
    fn set_github_sync(&self, on: bool);
    fn github_sync_now(&self);
    /// The kind of target chosen in the Sync page.
    fn set_sync_kind(&self, kind: &str);
    /// Connects a target other than GitHub (asks for the sync password).
    fn sync_connect(&self, target: crate::remote::Target, secret: String, done: Box<dyn Fn(Result<String, String>)>);
    /// Signs in to Nextcloud in the browser and connects `folder`.
    fn nextcloud_sign_in(&self, server: String, folder: String, done: Box<dyn Fn(Result<String, String>)>);
    fn sync_disconnect(&self);
    /// The connected target other than GitHub, for the status.
    fn sync_description(&self) -> Option<String>;
}

#[derive(Clone, Copy)]
pub enum PasswordAction {
    Set,
    Change,
    Remove,
}

/// A tab type with a configurable colour.
pub struct TabColor {
    pub key: &'static str,
    pub label: &'static str,
    pub default: &'static str,
}

const LANGUAGES: [&str; 3] = ["system", "en", "de"];

pub fn show(
    parent: &impl IsA<gtk::Widget>,
    settings: &Settings,
    app_password: bool,
    colors: Vec<TabColor>,
    handler: Rc<dyn Handler>,
) {
    let dialog = adw::PreferencesDialog::new();
    dialog.set_title(t!("Preferences"));
    dialog.set_search_enabled(true);
    // Room for all page titles; low enough to keep the window visible behind it.
    let sync = crate::features::on(crate::features::Feature::Sync);
    dialog.set_content_width(if sync { 1080 } else { 960 });
    dialog.set_content_height(620);
    dialog.add(&general_page(parent, settings, &handler));
    dialog.add(&features_page(settings, &handler));
    dialog.add(&themes_page(settings, &handler));
    dialog.add(&tabs_page(settings, colors, &handler));
    dialog.add(&security_page(&dialog, app_password, &handler));
    if sync {
        dialog.add(&sync_page(&dialog, settings, &handler));
    }
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

    // ----- Emmet -----------------------------------------------------------
    let emmet = adw::PreferencesGroup::builder()
        .title("Emmet")
        .description(t!("Abbreviations like ul>li*3 or m10 in HTML, XML, XSLT, CSS, SCSS and Less files."))
        .build();
    let tab = adw::SwitchRow::builder()
        .title(t!("Expand with Tab"))
        .subtitle(t!("Ctrl+E always expands; Ctrl+Shift+G wraps the selection"))
        .active(settings.emmet_tab)
        .build();
    let h = handler.clone();
    tab.connect_active_notify(move |row| h.set_emmet_tab(row.is_active()));
    emmet.add(&tab);
    page.add(&emmet);

    // ----- YAML style rules ---------------------------------------------
    if crate::features::on(crate::features::Feature::DataFormats) {
        let yaml = adw::PreferencesGroup::builder()
            .title(t!("YAML Checks"))
            .description(t!("Style rules for YAML files. Syntax errors, duplicate keys and schema problems are always shown."))
            .build();
        for (key, label, description) in crate::yaml_lint::RULES {
            let row = adw::SwitchRow::builder()
                .title(crate::i18n::tr(label))
                .subtitle(crate::i18n::tr(description))
                .active(settings.yaml_rules.get(key))
                .build();
            let h = handler.clone();
            row.connect_active_notify(move |row| h.set_yaml_rule(key, row.is_active()));
            yaml.add(&row);
        }
        page.add(&yaml);
    }

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

fn features_page(settings: &Settings, handler: &Rc<dyn Handler>) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::builder()
        .title(t!("Features"))
        .icon_name("application-x-addon-symbolic")
        .build();
    let group = adw::PreferencesGroup::builder()
        .title(t!("Optional Features"))
        .description(t!("Switched-off features disappear from menus, toolbar and help. Documents stay as they are: presentations and boards open as Markdown, charts show as code."))
        .build();
    for (feature, label, description) in crate::features::LIST {
        let row = adw::SwitchRow::builder()
            .title(crate::i18n::tr(label))
            .subtitle(crate::i18n::tr(description))
            .active(settings.features.get(*feature))
            .build();
        let (h, feature) = (handler.clone(), *feature);
        row.connect_active_notify(move |row| h.set_feature(feature, row.is_active()));
        group.add(&row);
    }
    page.add(&group);
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

fn security_page(dialog: &adw::PreferencesDialog, on: bool, handler: &Rc<dyn Handler>) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::builder()
        .title(t!("Security"))
        .icon_name("system-lock-screen-symbolic")
        .build();
    let group = adw::PreferencesGroup::builder()
        .title(t!("App Password"))
        .description(t!("With an app password QuickNotes asks for it at every start and stores your tabs and notes encrypted (Argon2id, XChaCha20-Poly1305). Files opened from disk stay as they are. A forgotten password cannot be recovered."))
        .build();
    let status = adw::ActionRow::builder().title(t!("App password")).build();
    let set = gtk::Button::builder().label(t!("Set…")).valign(gtk::Align::Center).build();
    set.add_css_class("suggested-action");
    let change = gtk::Button::builder().label(t!("Change…")).valign(gtk::Align::Center).build();
    let remove = gtk::Button::builder().label(t!("Remove…")).valign(gtk::Align::Center).build();
    remove.add_css_class("destructive-action");
    for b in [&set, &change, &remove] {
        status.add_suffix(b);
    }
    group.add(&status);
    let lock = button_row(t!("Lock Now"), t!("Show the password prompt instead of the notes"), "system-lock-screen-symbolic");
    group.add(&lock);
    page.add(&group);

    let show: Rc<dyn Fn(bool)> = {
        let (status, set, change, remove, lock) = (status.clone(), set.clone(), change.clone(), remove.clone(), lock.clone());
        Rc::new(move |on: bool| {
            status.set_subtitle(if on { t!("On – tabs and notes are stored encrypted") } else { t!("Off") });
            set.set_visible(!on);
            change.set_visible(on);
            remove.set_visible(on);
            lock.set_visible(on);
        })
    };
    show(on);
    for (button, action) in [(&set, PasswordAction::Set), (&change, PasswordAction::Change), (&remove, PasswordAction::Remove)] {
        let (h, show) = (handler.clone(), show.clone());
        button.connect_clicked(move |_| {
            let show = show.clone();
            h.app_password(action, Box::new(move |on| show(on)));
        });
    }
    let h = handler.clone();
    let dlg = dialog.downgrade();
    lock.connect_activated(move |_| {
        if let Some(d) = dlg.upgrade() {
            d.close();
        }
        h.lock_app();
    });
    page
}

/// Sync target kinds of the Sync page.
const SYNC_KINDS: [&str; 5] = ["github", "nextcloud", "webdav", "folder", "s3"];

fn sync_page(dialog: &adw::PreferencesDialog, settings: &Settings, handler: &Rc<dyn Handler>) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::builder()
        .title(t!("Sync"))
        .icon_name("emblem-synchronizing-symbolic")
        .build();
    let intro = adw::PreferencesGroup::builder()
        .description(t!("Notes, folders and the trash are synced encrypted with a sync password – the target sees nothing readable. Changes are fetched at start and every five minutes and sent when QuickNotes closes. One target at a time; files opened from disk are not synced."))
        .build();
    let labels = ["GitHub", "Nextcloud", "WebDAV", t!("Synced Folder"), t!("S3-compatible Storage")];
    let kind_row = adw::ComboRow::builder().title(t!("Sync With")).model(&gtk::StringList::new(&labels)).build();
    let current = match &settings.sync_target {
        Some(crate::remote::Target::Nextcloud { .. }) => "nextcloud",
        Some(crate::remote::Target::Webdav { .. }) => "webdav",
        Some(crate::remote::Target::Folder { .. }) => "folder",
        Some(crate::remote::Target::S3 { .. }) => "s3",
        None if SYNC_KINDS.contains(&settings.sync_kind.as_str()) => settings.sync_kind.as_str(),
        None => "github",
    };
    kind_row.set_selected(SYNC_KINDS.iter().position(|k| *k == current).unwrap_or(0) as u32);
    intro.add(&kind_row);
    page.add(&intro);

    let group = adw::PreferencesGroup::builder()
        .title("GitHub")
        .description(t!("A GitHub repository, private or public."))
        .build();

    let account = adw::ActionRow::builder().title(t!("Account")).build();
    let sign_in = gtk::Button::builder().label(t!("Sign In…")).valign(gtk::Align::Center).build();
    sign_in.add_css_class("suggested-action");
    sign_in.set_tooltip_text(Some(t!("Enter a code on github.com")));
    let token = gtk::Button::builder().label(t!("Use Token…")).valign(gtk::Align::Center).build();
    token.set_tooltip_text(Some(t!("Sign in with a personal access token")));
    let sign_out = gtk::Button::builder().label(t!("Sign Out")).valign(gtk::Align::Center).build();
    for b in [&sign_in, &token, &sign_out] {
        account.add_suffix(b);
    }
    group.add(&account);

    let repo = adw::EntryRow::builder()
        .title(t!("Repository (owner/name)"))
        .text(&settings.github_repo)
        .show_apply_button(true)
        .build();
    group.add(&repo);
    let status = adw::ActionRow::builder().title(t!("Status")).build();
    let disconnect = gtk::Button::builder().label(t!("Disconnect")).valign(gtk::Align::Center).build();
    disconnect.set_tooltip_text(Some(t!("Stop syncing with this repository (its data stays on GitHub)")));
    status.add_suffix(&disconnect);
    group.add(&status);

    let auto = adw::SwitchRow::builder()
        .title(t!("Sync Automatically"))
        .subtitle(t!("At start, every five minutes and when closing"))
        .active(settings.github_sync)
        .build();
    let h = handler.clone();
    auto.connect_active_notify(move |row| h.set_github_sync(row.is_active()));
    group.add(&auto);
    let now = button_row(t!("Sync Now"), t!("Fetch and send changes right away"), "emblem-synchronizing-symbolic");
    let h = handler.clone();
    now.connect_activated(move |_| h.github_sync_now());
    group.add(&now);
    page.add(&group);

    // The shown state; errors keep what was set before.
    #[derive(Clone, Default)]
    struct View {
        account: Option<String>,
        repo: Option<String>,
    }
    let view = Rc::new(std::cell::RefCell::new(View {
        account: Some(settings.github_account.clone()).filter(|a| !a.is_empty()),
        repo: Some(settings.github_repo.clone()).filter(|r| !r.is_empty() && !settings.github_account.is_empty()),
    }));
    type Show = Rc<dyn Fn(Option<&str>, Option<&str>)>;
    let show: Show = {
        let (account, sign_in, token, sign_out, repo, status, disconnect, auto, now, view) = (
            account.clone(),
            sign_in.clone(),
            token.clone(),
            sign_out.clone(),
            repo.clone(),
            status.clone(),
            disconnect.clone(),
            auto.clone(),
            now.clone(),
            view.clone(),
        );
        // Errors of the account (first) or the repository (second).
        Rc::new(move |account_error: Option<&str>, repo_error: Option<&str>| {
            let v = view.borrow().clone();
            let signed_in = v.account.is_some();
            let subtitle = match (&v.account, account_error) {
                (_, Some(e)) => format!("⚠ {}", glib::markup_escape_text(e)),
                (Some(a), None) => glib::markup_escape_text(&t!("Signed in as {}", a)).to_string(),
                (None, None) => t!("Not signed in").to_owned(),
            };
            account.set_subtitle(&subtitle);
            sign_in.set_visible(!crate::github::CLIENT_ID.is_empty());
            sign_in.set_label(if signed_in { t!("Sign In Again…") } else { t!("Sign In…") });
            sign_in.set_sensitive(true);
            token.set_sensitive(true);
            sign_out.set_visible(signed_in);
            repo.set_sensitive(signed_in);
            let subtitle = match (&v.repo, repo_error) {
                (_, Some(e)) => format!("⚠ {}", glib::markup_escape_text(e)),
                (Some(r), None) => glib::markup_escape_text(r).to_string(),
                (None, None) if signed_in => t!("Enter a repository and press Enter").to_owned(),
                (None, None) => t!("Sign in first").to_owned(),
            };
            status.set_subtitle(&subtitle);
            disconnect.set_visible(v.repo.is_some());
            auto.set_sensitive(v.repo.is_some());
            now.set_sensitive(v.repo.is_some());
        })
    };
    show(None, None);

    for (button, use_token) in [(&sign_in, false), (&token, true)] {
        let (h, show, view) = (handler.clone(), show.clone(), view.clone());
        let (sign_in, token) = (sign_in.clone(), token.clone());
        button.connect_clicked(move |_| {
            sign_in.set_sensitive(false);
            token.set_sensitive(false);
            let (show, view) = (show.clone(), view.clone());
            h.github_sign_in(
                use_token,
                Box::new(move |result| match result {
                    Ok(login) => {
                        view.borrow_mut().account = Some(login);
                        show(None, None);
                    }
                    Err(e) => show(Some(&e), None),
                }),
            );
        });
    }
    let (h, show_c, view_c, auto_c) = (handler.clone(), show.clone(), view.clone(), auto.clone());
    sign_out.connect_clicked(move |_| {
        h.github_sign_out();
        *view_c.borrow_mut() = View::default();
        auto_c.set_active(false);
        show_c(None, None);
    });
    let (h, show_c, view_c, auto_c) = (handler.clone(), show.clone(), view.clone(), auto.clone());
    repo.connect_apply(move |row| {
        row.set_sensitive(false);
        let (show, view, auto, row) = (show_c.clone(), view_c.clone(), auto_c.clone(), row.clone());
        h.github_connect(
            row.text().to_string(),
            Box::new(move |result| {
                row.set_sensitive(true);
                match result {
                    Ok(description) => {
                        view.borrow_mut().repo = Some(description);
                        auto.set_active(true);
                        show(None, None);
                    }
                    Err(e) => show(None, Some(&e)),
                }
            }),
        );
    });
    let (h, show_c, view_c, auto_c, repo_c) = (handler.clone(), show.clone(), view.clone(), auto.clone(), repo.clone());
    disconnect.connect_clicked(move |_| {
        h.github_disconnect();
        view_c.borrow_mut().repo = None;
        repo_c.set_text("");
        auto_c.set_active(false);
        show_c(None, None);
    });
    let github_group = group;

    // ----- other targets -----------------------------------------------------
    let nextcloud = adw::PreferencesGroup::builder()
        .title("Nextcloud")
        .description(t!("Sign in in the browser; Nextcloud creates an app password for QuickNotes."))
        .build();
    let nc_server = adw::EntryRow::builder().title(t!("Server Address")).build();
    let nc_folder = adw::EntryRow::builder().title(t!("Folder")).text("QuickNotes").build();
    let nc_sign_in = button_row(t!("Sign In in the Browser…"), t!("Then confirm the access in Nextcloud"), "web-browser-symbolic");
    for w in [nc_server.upcast_ref::<gtk::Widget>(), nc_folder.upcast_ref(), nc_sign_in.upcast_ref()] {
        nextcloud.add(w);
    }

    let webdav = adw::PreferencesGroup::builder()
        .title("WebDAV")
        .description(t!("Any WebDAV folder: ownCloud, Nextcloud with an app password, Synology, Hetzner Storage Box, mailbox.org, a NAS …"))
        .build();
    let dav_url = adw::EntryRow::builder().title(t!("Folder Address (URL)")).build();
    let dav_user = adw::EntryRow::builder().title(t!("User Name")).build();
    let dav_password = adw::PasswordEntryRow::builder().title(t!("Password")).build();
    let dav_connect = button_row(t!("Connect"), t!("The folder is created if it is missing"), "network-server-symbolic");
    for w in [dav_url.upcast_ref::<gtk::Widget>(), dav_user.upcast_ref(), dav_password.upcast_ref(), dav_connect.upcast_ref()] {
        webdav.add(w);
    }

    let folder = adw::PreferencesGroup::builder()
        .title(t!("Synced Folder"))
        .description(t!("A folder that another program syncs: Dropbox, OneDrive, Google Drive for desktop, pCloud, Syncthing, a USB stick …"))
        .build();
    let folder_row = adw::ActionRow::builder().title(t!("Folder")).subtitle(t!("Not chosen")).build();
    let choose = gtk::Button::builder().label(t!("Choose…")).valign(gtk::Align::Center).build();
    folder_row.add_suffix(&choose);
    folder.add(&folder_row);

    let s3 = adw::PreferencesGroup::builder()
        .title(t!("S3-compatible Storage"))
        .description(t!("A bucket at AWS, Backblaze B2, Wasabi, Cloudflare R2, MinIO …"))
        .build();
    let s3_endpoint = adw::EntryRow::builder().title(t!("Endpoint (empty for AWS)")).build();
    let s3_region = adw::EntryRow::builder().title(t!("Region")).text("us-east-1").build();
    let s3_bucket = adw::EntryRow::builder().title(t!("Bucket")).build();
    let s3_prefix = adw::EntryRow::builder().title(t!("Path in the Bucket (optional)")).build();
    let s3_access = adw::EntryRow::builder().title(t!("Access Key ID")).build();
    let s3_secret = adw::PasswordEntryRow::builder().title(t!("Secret Access Key")).build();
    let s3_connect = button_row(t!("Connect"), t!("The bucket must exist"), "network-server-symbolic");
    for w in [
        s3_endpoint.upcast_ref::<gtk::Widget>(),
        s3_region.upcast_ref(),
        s3_bucket.upcast_ref(),
        s3_prefix.upcast_ref(),
        s3_access.upcast_ref(),
        s3_secret.upcast_ref(),
        s3_connect.upcast_ref(),
    ] {
        s3.add(w);
    }
    // Fill in what is connected.
    match &settings.sync_target {
        Some(crate::remote::Target::Nextcloud { server, folder: f, .. }) => {
            nc_server.set_text(server);
            nc_folder.set_text(f);
        }
        Some(crate::remote::Target::Webdav { url, user }) => {
            dav_url.set_text(url);
            dav_user.set_text(user);
        }
        Some(crate::remote::Target::Folder { path }) => folder_row.set_subtitle(&glib::markup_escape_text(&path.display().to_string())),
        Some(crate::remote::Target::S3 { endpoint, region, bucket, prefix, access_key }) => {
            s3_endpoint.set_text(endpoint);
            s3_region.set_text(region);
            s3_bucket.set_text(bucket);
            s3_prefix.set_text(prefix);
            s3_access.set_text(access_key);
        }
        None => {}
    }

    let remote_status = adw::PreferencesGroup::new();
    let status_row = adw::ActionRow::builder().title(t!("Status")).build();
    let remote_disconnect = gtk::Button::builder().label(t!("Disconnect")).valign(gtk::Align::Center).build();
    remote_disconnect.set_tooltip_text(Some(t!("Stop syncing with this target (its data stays there)")));
    status_row.add_suffix(&remote_disconnect);
    let remote_auto = adw::SwitchRow::builder()
        .title(t!("Sync Automatically"))
        .subtitle(t!("At start, every five minutes and when closing"))
        .active(settings.github_sync && settings.sync_target.is_some())
        .build();
    let h = handler.clone();
    remote_auto.connect_active_notify(move |row| h.set_github_sync(row.is_active()));
    let remote_now = button_row(t!("Sync Now"), t!("Fetch and send changes right away"), "emblem-synchronizing-symbolic");
    let h = handler.clone();
    remote_now.connect_activated(move |_| h.github_sync_now());
    for w in [status_row.upcast_ref::<gtk::Widget>(), remote_auto.upcast_ref(), remote_now.upcast_ref()] {
        remote_status.add(w);
    }

    type Shown = Rc<std::cell::RefCell<Option<String>>>;
    let connected: Shown = Rc::new(std::cell::RefCell::new(
        settings.sync_target.as_ref().map(|t| format!("{}: {}", t.name(), t.describe())),
    ));
    type ShowRemote = Rc<dyn Fn(Option<&str>)>;
    let show_remote: ShowRemote = {
        let (status_row, remote_disconnect, remote_auto, remote_now, connected) =
            (status_row.clone(), remote_disconnect.clone(), remote_auto.clone(), remote_now.clone(), connected.clone());
        Rc::new(move |message: Option<&str>| {
            let c = connected.borrow().clone();
            let subtitle = match (message, &c) {
                (Some(m), _) => glib::markup_escape_text(m).to_string(),
                (None, Some(d)) => glib::markup_escape_text(&t!("Connected – {}", d)).to_string(),
                (None, None) => t!("Not connected").to_owned(),
            };
            status_row.set_subtitle(&subtitle);
            remote_disconnect.set_visible(c.is_some());
            remote_auto.set_sensitive(c.is_some());
            remote_now.set_sensitive(c.is_some());
        })
    };
    show_remote(None);

    // Connecting a target: the result in the status row; GitHub stops.
    type Connect = Rc<dyn Fn(crate::remote::Target, String)>;
    let connect: Connect = {
        let (h, show_remote, connected, remote_auto) = (handler.clone(), show_remote.clone(), connected.clone(), remote_auto.clone());
        let (github_view, github_show, github_auto, github_repo) = (view.clone(), show.clone(), auto.clone(), repo.clone());
        Rc::new(move |target, secret| {
            show_remote(Some(t!("Connecting…")));
            let (show_remote, connected, remote_auto) = (show_remote.clone(), connected.clone(), remote_auto.clone());
            let (github_view, github_show, github_auto, github_repo) =
                (github_view.clone(), github_show.clone(), github_auto.clone(), github_repo.clone());
            h.sync_connect(
                target,
                secret,
                Box::new(move |result| match result {
                    Ok(description) => {
                        *connected.borrow_mut() = Some(description);
                        remote_auto.set_active(true);
                        show_remote(None);
                        github_view.borrow_mut().repo = None;
                        github_repo.set_text("");
                        github_auto.set_active(false);
                        github_show(None, None);
                    }
                    Err(e) => show_remote(Some(&format!("⚠ {e}"))),
                }),
            );
        })
    };
    {
        let (h, show_remote, connected, remote_auto) = (handler.clone(), show_remote.clone(), connected.clone(), remote_auto.clone());
        let (server, folder_entry) = (nc_server.clone(), nc_folder.clone());
        let (github_view, github_show, github_repo) = (view.clone(), show.clone(), repo.clone());
        nc_sign_in.connect_activated(move |_| {
            if server.text().trim().is_empty() {
                show_remote(Some(&format!("⚠ {}", t!("Enter the address of your Nextcloud"))));
                return;
            }
            show_remote(Some(t!("Waiting for the sign-in in the browser…")));
            let (show_remote, connected, remote_auto) = (show_remote.clone(), connected.clone(), remote_auto.clone());
            let (github_view, github_show, github_repo) = (github_view.clone(), github_show.clone(), github_repo.clone());
            h.nextcloud_sign_in(
                server.text().to_string(),
                folder_entry.text().to_string(),
                Box::new(move |result| match result {
                    Ok(description) => {
                        *connected.borrow_mut() = Some(description);
                        remote_auto.set_active(true);
                        show_remote(None);
                        github_view.borrow_mut().repo = None;
                        github_repo.set_text("");
                        github_show(None, None);
                    }
                    Err(e) => show_remote(Some(&format!("⚠ {e}"))),
                }),
            );
        });
    }
    {
        let (connect, show_remote) = (connect.clone(), show_remote.clone());
        let (url, user, password) = (dav_url.clone(), dav_user.clone(), dav_password.clone());
        dav_connect.connect_activated(move |_| {
            let u = url.text().trim().to_owned();
            if !(u.starts_with("https://") || u.starts_with("http://")) {
                show_remote(Some(&format!("⚠ {}", t!("Enter the address of a folder (https://…)"))));
                return;
            }
            let u = if u.ends_with('/') { u } else { format!("{u}/") };
            connect(crate::remote::Target::Webdav { url: u, user: user.text().trim().to_owned() }, password.text().to_string());
        });
    }
    {
        let (connect, row) = (connect.clone(), folder_row.clone());
        let dialog = dialog.downgrade();
        choose.connect_clicked(move |_| {
            let chooser = gtk::FileDialog::builder().title(t!("Synced Folder")).modal(true).build();
            let parent = dialog.upgrade().and_then(|d| d.root()).and_downcast::<gtk::Window>();
            let (connect, row) = (connect.clone(), row.clone());
            chooser.select_folder(parent.as_ref(), None::<&gtk::gio::Cancellable>, move |result| {
                let Some(path) = result.ok().and_then(|f| f.path()) else { return };
                row.set_subtitle(&glib::markup_escape_text(&path.display().to_string()));
                connect(crate::remote::Target::Folder { path }, String::new());
            });
        });
    }
    {
        let (connect, show_remote) = (connect.clone(), show_remote.clone());
        let fields = (s3_endpoint.clone(), s3_region.clone(), s3_bucket.clone(), s3_prefix.clone(), s3_access.clone(), s3_secret.clone());
        s3_connect.connect_activated(move |_| {
            let (endpoint, region, bucket, prefix, access, secret) = &fields;
            if bucket.text().trim().is_empty() || access.text().trim().is_empty() || secret.text().is_empty() {
                show_remote(Some(&format!("⚠ {}", t!("Enter bucket, access key ID and secret access key"))));
                return;
            }
            connect(
                crate::remote::Target::S3 {
                    endpoint: endpoint.text().trim().trim_end_matches('/').to_owned(),
                    region: region.text().trim().to_owned(),
                    bucket: bucket.text().trim().to_owned(),
                    prefix: prefix.text().trim().trim_matches('/').to_owned(),
                    access_key: access.text().trim().to_owned(),
                },
                secret.text().to_string(),
            );
        });
    }
    {
        let (h, show_remote, connected, remote_auto) = (handler.clone(), show_remote.clone(), connected.clone(), remote_auto.clone());
        remote_disconnect.connect_clicked(move |_| {
            h.sync_disconnect();
            *connected.borrow_mut() = None;
            remote_auto.set_active(false);
            show_remote(None);
        });
    }
    let groups = [github_group.clone(), nextcloud.clone(), webdav.clone(), folder.clone(), s3.clone()];
    for g in &groups {
        page.add(g);
    }
    page.add(&remote_status);
    let sync_visible = {
        let (groups, remote_status) = (groups.clone(), remote_status.clone());
        move |index: u32| {
            for (i, g) in groups.iter().enumerate() {
                g.set_visible(i as u32 == index);
            }
            remote_status.set_visible(index != 0);
        }
    };
    sync_visible(kind_row.selected());
    let h = handler.clone();
    kind_row.connect_selected_notify(move |row| {
        sync_visible(row.selected());
        h.set_sync_kind(SYNC_KINDS.get(row.selected() as usize).copied().unwrap_or("github"));
        // Connecting GitHub meanwhile ended the other target.
        *connected.borrow_mut() = h.sync_description();
        show_remote(None);
    });
    page
}

fn import_export_page(dialog: &adw::PreferencesDialog, handler: &Rc<dyn Handler>) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::builder()
        .title(t!("Import/Export"))
        .icon_name("document-save-symbolic")
        .build();

    let backup = adw::PreferencesGroup::builder()
        .title(t!("Backup"))
        .description(t!("All tabs, notes and settings in one file, optionally encrypted with a password."))
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
