//! Persistence of the session (open tabs, settings) and note contents.
//!
//! Layout below `$XDG_DATA_HOME/quicknotes` (usually `~/.local/share/quicknotes`):
//! - `session.json`   tab order, per-tab metadata and global settings
//! - `notes/<id>.txt` content of notes and of unsaved changes to opened files

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub font_size: f64,
    pub font_family: String,
    /// OpenType ligatures (e.g. `!=` as `≠` in Fira Code).
    pub ligatures: bool,
    /// Kept in sync with the theme (fallback when a theme no longer exists).
    pub dark: bool,
    /// Colour theme id, see `themes.rs`; empty: default for `dark`.
    pub theme: String,
    pub width: i32,
    pub height: i32,
    pub maximized: bool,
    /// Command used to render PlantUML, e.g. `plantuml` or `java -jar /opt/plantuml.jar`.
    /// Auto-detected when empty.
    pub plantuml_command: Option<String>,
    /// Files larger than this are opened without syntax highlighting.
    pub large_file_mb: u64,
    /// Last used options of the Geocaching tools, per tool id.
    pub gc_options: std::collections::BTreeMap<String, crate::geocaching::Options>,
    /// User interface language: "system", "en" or "de".
    pub language: String,
    /// Tabs in a sidebar on the left instead of the tab bar on top.
    pub vertical_tabs: bool,
    /// Width of the tab sidebar in pixels.
    pub sidebar_width: i32,
    /// User-defined tab colours (`#rrggbb`) per tab type, e.g. `code`.
    pub tab_colors: std::collections::BTreeMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            font_size: 12.0,
            font_family: "Monospace".into(),
            ligatures: true,
            dark: true,
            theme: String::new(),
            width: 1100,
            height: 750,
            maximized: false,
            plantuml_command: None,
            large_file_mb: 8,
            gc_options: Default::default(),
            language: "system".into(),
            vertical_tabs: false,
            sidebar_width: 220,
            tab_colors: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TabKind {
    #[default]
    Text,
    Image,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct TabMeta {
    pub id: String,
    pub kind: TabKind,
    /// User-defined tab title; derived from content/file name when `None`.
    pub title: Option<String>,
    /// Backing file, if the tab was opened from or saved to disk.
    pub path: Option<PathBuf>,
    pub language: Option<String>,
    pub language_locked: bool,
    pub wrap: bool,
    /// Content differs from the backing file (content is then stored in `notes/`).
    pub modified: bool,
    pub preview: bool,
    pub cursor: i32,
    /// Editor share of the editor/preview split (0..1); centred when `None`.
    pub split: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(default)]
pub struct Session {
    pub version: u32,
    pub active: usize,
    pub tabs: Vec<TabMeta>,
    pub settings: Settings,
}

pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".local/share"))
        .join("quicknotes")
}

fn notes_dir() -> PathBuf {
    data_dir().join("notes")
}

fn note_path(id: &str) -> PathBuf {
    notes_dir().join(format!("{id}.txt"))
}

fn session_path() -> PathBuf {
    data_dir().join("session.json")
}

/// Writes via a temporary file and rename so a crash never leaves a truncated file.
fn write_atomic(path: &Path, data: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)
}

pub fn load_session() -> Session {
    let path = session_path();
    match fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
            eprintln!("quicknotes: cannot parse {}: {e}", path.display());
            // Keep the broken file for manual recovery.
            let _ = fs::copy(&path, path.with_extension("json.broken"));
            Session::default()
        }),
        Err(_) => Session::default(),
    }
}

pub fn save_session(session: &Session) -> io::Result<()> {
    let json = serde_json::to_vec_pretty(session).map_err(io::Error::other)?;
    write_atomic(&session_path(), &json)
}

pub fn read_note(id: &str) -> Option<String> {
    fs::read(note_path(id))
        .ok()
        .map(|b| String::from_utf8(b).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned()))
}

pub fn write_note(id: &str, content: &str) -> io::Result<()> {
    write_atomic(&note_path(id), content.as_bytes())
}

pub fn delete_note(id: &str) {
    let _ = fs::remove_file(note_path(id));
}

pub fn new_id() -> String {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    format!("{nanos:x}{:02x}", COUNTER.fetch_add(1, Ordering::Relaxed) & 0xff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_tolerates_missing_fields() {
        let s: Session = serde_json::from_str(r#"{"tabs":[{"id":"a"}]}"#).unwrap();
        assert_eq!(s.tabs[0].kind, TabKind::Text);
        assert_eq!(s.settings.font_size, 12.0);
    }

    #[test]
    fn ids_are_unique() {
        assert_ne!(new_id(), new_id());
    }
}
