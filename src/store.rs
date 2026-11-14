//! Persistence of the session (open tabs, settings) and note contents.
//!
//! Layout below `$XDG_DATA_HOME/quicknotes` (usually `~/.local/share/quicknotes`):
//! - `session.json`   tab order, per-tab metadata and global settings
//! - `notes/<id>.txt` content of notes and of unsaved changes to opened files
//!
//! With the app password on, the tabs part of the session is stored sealed in
//! `session.json` (settings stay readable for the unlock screen) and notes are
//! stored as `notes/<id>.enc`, both encrypted with the app's data key.

use crate::crypto::{self, Key, Lock};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
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
    /// Tab expands Emmet abbreviations in HTML, XML and CSS files.
    pub emmet_tab: bool,
    /// Formatting toolbar above Markdown editors.
    pub format_bar: bool,
    /// Optional features that are on (all of them for settings saved
    /// before the switches existed, none for new installations).
    #[serde(default = "crate::features::Features::all")]
    pub features: crate::features::Features,
    /// The notes sidebar (folders) is shown.
    pub notes_sidebar: bool,
    pub notes_sidebar_width: i32,
    /// GitHub sync: signed-in account (login); empty when signed out. The
    /// token is in the keyring.
    pub github_account: String,
    /// GitHub sync: repository (`owner/name`); empty when none is set up.
    pub github_repo: String,
    /// GitHub sync: sync at start and when closing.
    pub github_sync: bool,
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
            emmet_tab: true,
            format_bar: true,
            features: Default::default(),
            notes_sidebar: true,
            notes_sidebar_width: 240,
            github_account: String::new(),
            github_repo: String::new(),
            github_sync: false,
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
    /// Tab password: the tab's data key, wrapped with the password's key.
    /// The content is then stored in `notes/<id>.locked`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock: Option<Lock>,
    /// Folder of the notes library (see `library.rs`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    /// In the trash (closed notes only).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub trashed: bool,
    /// Title shown in the library while the note is closed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// A folder of the notes library.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(default)]
pub struct Folder {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub collapsed: bool,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct Session {
    pub version: u32,
    pub active: usize,
    pub tabs: Vec<TabMeta>,
    /// Closed notes and file links of the notes library.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub library: Vec<TabMeta>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub folders: Vec<Folder>,
    pub settings: Settings,
    /// App password: the data key, wrapped with the password's key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vault: Option<Lock>,
    /// With the app password: `active` and `tabs`, encrypted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sealed: Option<String>,
}

/// The encrypted part of a session.
#[derive(Serialize, Deserialize, Default)]
struct SealedTabs {
    active: usize,
    tabs: Vec<TabMeta>,
    #[serde(default)]
    library: Vec<TabMeta>,
    #[serde(default)]
    folders: Vec<Folder>,
}

const SESSION_AAD: &[u8] = b"quicknotes-session";

thread_local! {
    /// The app's data key while the app password is on (and entered).
    static APP_KEY: RefCell<Option<Key>> = const { RefCell::new(None) };
}

pub fn set_app_key(key: Option<Key>) {
    APP_KEY.with(|k| *k.borrow_mut() = key);
}

pub fn app_key() -> Option<Key> {
    APP_KEY.with(|k| k.borrow().clone())
}

impl Session {
    /// Decrypts the tabs of a locked session.
    pub fn unseal(&mut self, key: &Key) -> Result<(), crypto::Error> {
        let Some(sealed) = &self.sealed else { return Ok(()) };
        let json = key.open_text(sealed, SESSION_AAD)?;
        let part: SealedTabs = serde_json::from_str(&json).map_err(|_| crypto::Error::Format)?;
        self.active = part.active;
        self.tabs = part.tabs;
        self.library = part.library;
        self.folders = part.folders;
        self.sealed = None;
        Ok(())
    }
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
    let json = match (&session.vault, app_key()) {
        (Some(_), Some(key)) => {
            let part = SealedTabs {
                active: session.active,
                tabs: session.tabs.clone(),
                library: session.library.clone(),
                folders: session.folders.clone(),
            };
            let plain = zeroize::Zeroizing::new(serde_json::to_string(&part).map_err(io::Error::other)?);
            let sealed = Session {
                version: session.version,
                active: 0,
                tabs: Vec::new(),
                library: Vec::new(),
                folders: Vec::new(),
                settings: session.settings.clone(),
                vault: session.vault.clone(),
                sealed: Some(key.seal_text(&plain, SESSION_AAD)),
            };
            serde_json::to_vec_pretty(&sealed)
        }
        _ => serde_json::to_vec_pretty(session),
    }
    .map_err(io::Error::other)?;
    write_atomic(&session_path(), &json)
}

fn sealed_note_path(id: &str) -> PathBuf {
    notes_dir().join(format!("{id}.enc"))
}

fn locked_note_path(id: &str) -> PathBuf {
    notes_dir().join(format!("{id}.locked"))
}

/// Content of a password-protected tab (sealed with the tab's key).
pub fn read_locked(id: &str) -> Option<Vec<u8>> {
    fs::read(locked_note_path(id)).ok()
}

pub fn write_locked(id: &str, sealed: &[u8]) -> io::Result<()> {
    write_atomic(&locked_note_path(id), sealed)?;
    let _ = fs::remove_file(note_path(id));
    let _ = fs::remove_file(sealed_note_path(id));
    Ok(())
}

pub fn read_note(id: &str) -> Option<String> {
    if let Ok(data) = fs::read(sealed_note_path(id)) {
        let key = app_key()?;
        return match key.open(&data, id.as_bytes()) {
            Ok(plain) => Some(String::from_utf8_lossy(&plain).into_owned()),
            Err(e) => {
                eprintln!("quicknotes: cannot decrypt note {id}: {e}");
                None
            }
        };
    }
    fs::read(note_path(id))
        .ok()
        .map(|b| String::from_utf8(b).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned()))
}

/// Stores a note, encrypted while the app password is on.
pub fn write_note(id: &str, content: &str) -> io::Result<()> {
    match app_key() {
        Some(key) => {
            write_atomic(&sealed_note_path(id), &key.seal(content.as_bytes(), id.as_bytes()))?;
            let _ = fs::remove_file(note_path(id));
        }
        None => {
            write_atomic(&note_path(id), content.as_bytes())?;
            let _ = fs::remove_file(sealed_note_path(id));
        }
    }
    let _ = fs::remove_file(locked_note_path(id));
    Ok(())
}

pub fn delete_note(id: &str) {
    let _ = fs::remove_file(note_path(id));
    let _ = fs::remove_file(sealed_note_path(id));
    let _ = fs::remove_file(locked_note_path(id));
}

/// Re-stores every note file with the current app key (or in plain text
/// without one), also notes of no open tab. Returns the number of failures.
pub fn convert_notes(old_key: Option<&Key>) -> usize {
    let Ok(entries) = fs::read_dir(notes_dir()) else { return 0 };
    let mut failed = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        let (Some(id), Some(ext)) = (path.file_stem().and_then(|s| s.to_str()), path.extension().and_then(|s| s.to_str()))
        else {
            continue;
        };
        let content = match (ext, old_key) {
            ("txt", _) => fs::read(&path).ok().map(|b| String::from_utf8_lossy(&b).into_owned()),
            ("enc", Some(key)) => fs::read(&path)
                .ok()
                .and_then(|data| key.open(&data, id.as_bytes()).ok())
                .map(|plain| String::from_utf8_lossy(&plain).into_owned()),
            _ => continue,
        };
        match content {
            Some(c) => {
                if write_note(id, &c).is_err() {
                    failed += 1;
                }
            }
            None => failed += 1,
        }
    }
    failed
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
    fn sealed_sessions() {
        let key = Key::random();
        let (lock, _) = Lock::create("pw", crate::crypto::tests::fast_kdf());
        let mut session = Session { vault: Some(lock), ..Default::default() };
        session.tabs.push(TabMeta { id: "a1".into(), title: Some("Secret plans".into()), ..Default::default() });
        session.active = 0;
        session.library.push(TabMeta { id: "b2".into(), label: Some("Closed diary".into()), ..Default::default() });
        session.folders.push(Folder { id: "f1".into(), name: "Private folder".into(), ..Default::default() });
        let part = SealedTabs {
            active: 0,
            tabs: session.tabs.clone(),
            library: session.library.clone(),
            folders: session.folders.clone(),
        };
        let sealed = Session {
            sealed: Some(key.seal_text(&serde_json::to_string(&part).unwrap(), SESSION_AAD)),
            tabs: Vec::new(),
            library: Vec::new(),
            folders: Vec::new(),
            ..session.clone()
        };
        let json = serde_json::to_string(&sealed).unwrap();
        assert!(!json.contains("Secret plans") && !json.contains("Closed diary") && !json.contains("Private folder"));
        let mut loaded: Session = serde_json::from_str(&json).unwrap();
        assert!(loaded.tabs.is_empty());
        assert_eq!(loaded.clone().unseal(&Key::random()), Err(crate::crypto::Error::Decrypt));
        loaded.unseal(&key).unwrap();
        assert_eq!(loaded.tabs[0].title.as_deref(), Some("Secret plans"));
        assert_eq!(loaded.library[0].label.as_deref(), Some("Closed diary"));
        assert_eq!(loaded.folders[0].name, "Private folder");
        assert!(loaded.sealed.is_none());
    }

    #[test]
    fn ids_are_unique() {
        assert_ne!(new_id(), new_id());
    }
}
