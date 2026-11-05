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

pub fn register(display: &gdk::Display) {
    let base = store::data_dir().join("icons");
    for (size, data) in ICONS {
        let dir = base.join(format!("hicolor/{size}x{size}/apps"));
        let file = dir.join(format!("{APP_ID}.png"));
        if fs::read(&file).ok().as_deref() != Some(*data) {
            let _ = fs::create_dir_all(&dir);
            let _ = fs::write(&file, data);
        }
    }
    gtk::IconTheme::for_display(display).add_search_path(&base);
    gtk::Window::set_default_icon_name(APP_ID);
}
