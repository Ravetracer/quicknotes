//! Bundled application icon.
//!
//! The PNGs are compiled into the binary and registered with the icon theme
//! at startup, so window icon, About dialog etc. work without installation.

use crate::{store, APP_ID};
use gtk::gdk;
use std::fs;

const ICONS: &[(u32, &[u8])] = &[
    (16, include_bytes!("../data/icons/hicolor/16x16/apps/io.github.quicknotes.png")),
    (24, include_bytes!("../data/icons/hicolor/24x24/apps/io.github.quicknotes.png")),
    (32, include_bytes!("../data/icons/hicolor/32x32/apps/io.github.quicknotes.png")),
    (48, include_bytes!("../data/icons/hicolor/48x48/apps/io.github.quicknotes.png")),
    (64, include_bytes!("../data/icons/hicolor/64x64/apps/io.github.quicknotes.png")),
    (128, include_bytes!("../data/icons/hicolor/128x128/apps/io.github.quicknotes.png")),
    (256, include_bytes!("../data/icons/hicolor/256x256/apps/io.github.quicknotes.png")),
    (512, include_bytes!("../data/icons/hicolor/512x512/apps/io.github.quicknotes.png")),
];

/// Symbolic icons of the Markdown formatting toolbar (same look in every icon theme).
const FORMAT_ICONS: &[(&str, &[u8])] = &[
    ("bold", include_bytes!("../data/icons/format/qn-format-bold-symbolic.svg")),
    ("bullet-list", include_bytes!("../data/icons/format/qn-format-bullet-list-symbolic.svg")),
    ("chart", include_bytes!("../data/icons/format/qn-format-chart-symbolic.svg")),
    ("checklist", include_bytes!("../data/icons/format/qn-format-checklist-symbolic.svg")),
    ("code", include_bytes!("../data/icons/format/qn-format-code-symbolic.svg")),
    ("code-block", include_bytes!("../data/icons/format/qn-format-code-block-symbolic.svg")),
    ("color", include_bytes!("../data/icons/format/qn-format-color-symbolic.svg")),
    ("heading", include_bytes!("../data/icons/format/qn-format-heading-symbolic.svg")),
    ("image", include_bytes!("../data/icons/format/qn-format-image-symbolic.svg")),
    ("italic", include_bytes!("../data/icons/format/qn-format-italic-symbolic.svg")),
    ("math", include_bytes!("../data/icons/format/qn-format-math-symbolic.svg")),
    ("music", include_bytes!("../data/icons/format/qn-format-music-symbolic.svg")),
    ("link", include_bytes!("../data/icons/format/qn-format-link-symbolic.svg")),
    ("numbered-list", include_bytes!("../data/icons/format/qn-format-numbered-list-symbolic.svg")),
    ("quote", include_bytes!("../data/icons/format/qn-format-quote-symbolic.svg")),
    ("rule", include_bytes!("../data/icons/format/qn-format-rule-symbolic.svg")),
    ("strikethrough", include_bytes!("../data/icons/format/qn-format-strikethrough-symbolic.svg")),
    ("table", include_bytes!("../data/icons/format/qn-format-table-symbolic.svg")),
];

/// Icon name of a formatting toolbar icon, e.g. `qn-format-bold-symbolic`.
pub fn format_icon(name: &str) -> String {
    format!("qn-format-{name}-symbolic")
}

fn write_if_changed(file: &std::path::Path, data: &[u8]) {
    if fs::read(file).ok().as_deref() != Some(data) {
        if let Some(dir) = file.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = fs::write(file, data);
    }
}

pub fn register(display: &gdk::Display) {
    let base = store::data_dir().join("icons");
    // In a theme directory, as GTK does not recolour unthemed symbolic icons.
    for (name, data) in FORMAT_ICONS {
        write_if_changed(&base.join(format!("hicolor/scalable/actions/{}.svg", format_icon(name))), data);
    }
    for (size, data) in ICONS {
        write_if_changed(&base.join(format!("hicolor/{size}x{size}/apps/{APP_ID}.png")), data);
    }
    gtk::IconTheme::for_display(display).add_search_path(&base);
    gtk::Window::set_default_icon_name(APP_ID);
}
