//! Backup archives (`.tar.gz`) of all notes and the session.
//!
//! Layout:
//! - `manifest.json`  format version, app version, creation time
//! - `session.json`   tabs and settings (same format as the live session)
//! - `notes/<id>.txt` contents of notes and of unsaved changes to opened files
//! - `notes/<id>.locked` password-protected tabs, sealed with their own key
//!
//! Archives are only read into memory, never extracted to disk, so entry
//! names cannot write anywhere. Unknown entries are ignored.
//!
//! Encrypted backups (`.qnbackup`): `QNBK` | header length (u32 LE) | header
//! (JSON: Argon2id settings, or none when sealed with the app's data key) |
//! the `.tar.gz` sealed with [`crate::crypto`].

use crate::crypto::{self, Kdf, Key};
use crate::store::Session;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

const FORMAT: u32 = 1;
const SEALED_MAGIC: &[u8; 4] = b"QNBK";
const SEALED_AAD: &[u8] = b"quicknotes-backup";

/// How a backup file is encrypted.
pub enum Seal {
    /// With a password chosen for the backup (key derived with `kdf`).
    Password { key: Key, kdf: Kdf },
    /// With the app's data key (automatic backups while the app password is on).
    AppKey(Key),
}

#[derive(Serialize, Deserialize)]
struct SealHeader {
    format: u32,
    /// `None`: sealed with the app's data key.
    kdf: Option<Kdf>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReadError {
    /// Encrypted with a password; ask for it.
    NeedsPassword,
    WrongPassword,
    Failed(String),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::NeedsPassword => write!(f, "{}", t!("The backup is encrypted")),
            ReadError::WrongPassword => write!(f, "{}", t!("Wrong password")),
            ReadError::Failed(e) => write!(f, "{e}"),
        }
    }
}

impl From<String> for ReadError {
    fn from(e: String) -> Self {
        ReadError::Failed(e)
    }
}
/// Limits against corrupt or hostile archives (a backup is text only).
const MAX_ENTRY: u64 = 256 * 1024 * 1024;
const MAX_TOTAL: u64 = 1024 * 1024 * 1024;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct Manifest {
    pub format: u32,
    pub app_version: String,
    /// ISO 8601 creation time.
    pub created: String,
}

#[derive(Debug)]
pub struct Backup {
    pub manifest: Manifest,
    pub session: Session,
    /// Note contents by tab id.
    pub notes: HashMap<String, String>,
    /// Sealed contents of password-protected tabs by tab id.
    pub locked: HashMap<String, Vec<u8>>,
    /// Learning progress of flash cards (JSON).
    pub learning: Option<Vec<u8>>,
}

/// What goes into a backup besides the session.
#[derive(Default)]
pub struct Contents {
    pub notes: Vec<(String, String)>,
    pub locked: Vec<(String, Vec<u8>)>,
    /// Learning progress of flash cards (JSON).
    pub learning: Option<Vec<u8>>,
}

impl From<&Backup> for Contents {
    fn from(b: &Backup) -> Self {
        Contents {
            notes: b.notes.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            locked: b.locked.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            learning: b.learning.clone(),
        }
    }
}

/// Writes a backup to `path`, encrypted when `seal` is given.
pub fn write(
    path: &Path,
    created: &str,
    session: &Session,
    contents: &Contents,
    seal: Option<&Seal>,
) -> Result<(), String> {
    let err = |e: std::io::Error| format!("{}: {e}", path.display());
    let archive = archive(created, session, contents).map_err(err)?;
    let data = match seal {
        None => archive,
        Some(seal) => {
            let (key, kdf) = match seal {
                Seal::Password { key, kdf } => (key, Some(kdf.clone())),
                Seal::AppKey(key) => (key, None),
            };
            let header = serde_json::to_vec(&SealHeader { format: FORMAT, kdf }).map_err(|e| e.to_string())?;
            let mut out = SEALED_MAGIC.to_vec();
            out.extend_from_slice(&(header.len() as u32).to_le_bytes());
            out.extend_from_slice(&header);
            out.extend_from_slice(&key.seal(&archive, SEALED_AAD));
            out
        }
    };
    let mut file = File::create(path).map_err(err)?;
    file.write_all(&data).map_err(err)?;
    file.flush().map_err(err)?;
    Ok(())
}

/// The `.tar.gz` archive in memory.
fn archive(created: &str, session: &Session, contents: &Contents) -> std::io::Result<Vec<u8>> {
    let mut tar = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::default()));
    let mut add = |name: &str, data: &[u8]| -> std::io::Result<()> {
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o600);
        header.set_mtime(now_unix());
        header.set_cksum();
        tar.append_data(&mut header, name, data)
    };
    let manifest = Manifest {
        format: FORMAT,
        app_version: env!("CARGO_PKG_VERSION").into(),
        created: created.into(),
    };
    add("manifest.json", json(&manifest).as_bytes())?;
    add("session.json", json(session).as_bytes())?;
    for (id, content) in &contents.notes {
        if valid_id(id) {
            add(&format!("notes/{id}.txt"), content.as_bytes())?;
        }
    }
    for (id, sealed) in &contents.locked {
        if valid_id(id) {
            add(&format!("notes/{id}.locked"), sealed)?;
        }
    }
    if let Some(learning) = &contents.learning {
        add("learning.json", learning)?;
    }
    tar.into_inner()?.finish()
}

/// The file is an encrypted backup.
#[cfg(test)]
fn is_sealed(path: &Path) -> bool {
    let mut magic = [0u8; 4];
    File::open(path).and_then(|mut f| f.read_exact(&mut magic)).is_ok() && &magic == SEALED_MAGIC
}

/// Reads and validates a backup. Encrypted ones need `password` (or
/// `app_key` for automatic backups); deriving the key is slow, so run this
/// off the main thread.
pub fn read(path: &Path, password: Option<&str>, app_key: Option<&Key>) -> Result<Backup, ReadError> {
    let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !data.starts_with(SEALED_MAGIC) {
        return parse(&data);
    }
    let damaged = || ReadError::Failed(t!("The encrypted backup is damaged").into());
    let len = data.get(4..8).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize).ok_or_else(damaged)?;
    let header: SealHeader =
        data.get(8..8 + len).and_then(|h| serde_json::from_slice(h).ok()).ok_or_else(damaged)?;
    if header.format > FORMAT {
        return Err(t!("The backup was created by a newer QuickNotes version").to_owned().into());
    }
    let sealed = &data[8 + len..];
    let key = match (&header.kdf, password, app_key) {
        (Some(kdf), Some(pw), _) => kdf.derive(pw).map_err(|_| damaged())?,
        (Some(_), None, _) => return Err(ReadError::NeedsPassword),
        (None, _, Some(key)) => key.clone(),
        (None, _, None) => {
            return Err(t!("This automatic backup can only be restored while the app password is set").to_owned().into())
        }
    };
    let archive = key.open(sealed, SEALED_AAD).map_err(|e| match e {
        crypto::Error::Decrypt => ReadError::WrongPassword,
        crypto::Error::Format => damaged(),
    })?;
    parse(&archive)
}

fn parse(data: &[u8]) -> Result<Backup, ReadError> {
    let mut archive = tar::Archive::new(GzDecoder::new(data));
    let not_backup = || t!("This is not a QuickNotes backup (.tar.gz)").to_owned();
    let mut manifest = None;
    let mut session = None;
    let mut notes = HashMap::new();
    let mut locked = HashMap::new();
    let mut learning = None;
    let mut total = 0u64;

    for entry in archive.entries().map_err(|_| not_backup())? {
        let mut entry = entry.map_err(|_| not_backup())?;
        if entry.header().entry_type() != tar::EntryType::Regular {
            continue;
        }
        let name = entry.path().map_err(|_| not_backup())?.to_string_lossy().into_owned();
        let size = entry.header().size().unwrap_or(0);
        total += size;
        if size > MAX_ENTRY || total > MAX_TOTAL {
            return Err(t!("The backup is unexpectedly large").to_owned().into());
        }
        let mut data = Vec::with_capacity(size as usize);
        entry.by_ref().take(MAX_ENTRY).read_to_end(&mut data).map_err(|_| not_backup())?;
        let text = || String::from_utf8_lossy(&data).into_owned();
        match name.as_str() {
            "manifest.json" => {
                manifest = Some(serde_json::from_slice::<Manifest>(&data).map_err(|_| not_backup())?);
            }
            "session.json" => {
                session = Some(
                    serde_json::from_slice::<Session>(&data)
                        .map_err(|e| t!("The backup's session is damaged: {}", e))?,
                );
            }
            "learning.json" => learning = Some(data.clone()),
            n => {
                if let Some(id) = n.strip_prefix("notes/").and_then(|f| f.strip_suffix(".txt")) {
                    if valid_id(id) {
                        notes.insert(id.to_owned(), text());
                    }
                } else if let Some(id) = n.strip_prefix("notes/").and_then(|f| f.strip_suffix(".locked")) {
                    if valid_id(id) {
                        locked.insert(id.to_owned(), data.clone());
                    }
                }
            }
        }
    }

    let manifest = manifest.ok_or_else(not_backup)?;
    if manifest.format > FORMAT {
        return Err(t!("The backup was created by a newer QuickNotes version").to_owned().into());
    }
    let session = session.ok_or_else(not_backup)?;
    Ok(Backup { manifest, session, notes, locked, learning })
}

/// Tab ids are lowercase hex (see `store::new_id`).
fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_hexdigit())
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn json<T: Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::TabMeta;

    fn tmp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("qn-test-{}-{name}", std::process::id()))
    }

    #[test]
    fn roundtrip() {
        let path = tmp("roundtrip.tar.gz");
        let mut session = Session::default();
        session.tabs.push(TabMeta { id: "abc1".into(), ..Default::default() });
        session.settings.font_family = "Fira Code".into();
        let contents = Contents {
            notes: vec![("abc1".to_owned(), "hello\nwörld".to_owned())],
            locked: vec![("abc2".to_owned(), vec![1, 2, 3])],
            learning: Some(b"{\"cards\":{}}".to_vec()),
        };
        write(&path, "2026-11-05T12:00:00", &session, &contents, None).unwrap();
        assert!(!is_sealed(&path));

        let backup = read(&path, None, None).unwrap();
        assert_eq!(backup.manifest.format, FORMAT);
        assert_eq!(backup.learning.as_deref(), Some(&b"{\"cards\":{}}"[..]));
        assert_eq!(backup.manifest.created, "2026-11-05T12:00:00");
        assert_eq!(backup.session.tabs[0].id, "abc1");
        assert_eq!(backup.session.settings.font_family, "Fira Code");
        assert_eq!(backup.notes["abc1"], "hello\nwörld");
        assert_eq!(backup.locked["abc2"], [1, 2, 3]);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn encrypted_backups() {
        let path = tmp("sealed.qnbackup");
        let mut session = Session::default();
        session.tabs.push(TabMeta { id: "abc1".into(), title: Some("Secret title".into()), ..Default::default() });
        let notes = Contents { notes: vec![("abc1".to_owned(), "the secret note".to_owned())], locked: Vec::new(), learning: None };
        let kdf = crate::crypto::tests::fast_kdf();
        let seal = Seal::Password { key: kdf.derive("pw").unwrap(), kdf };
        write(&path, "2026-11-12T10:00:00", &session, &notes, Some(&seal)).unwrap();
        assert!(is_sealed(&path));
        let raw = std::fs::read(&path).unwrap();
        assert!(!raw.windows(6).any(|w| w == b"secret"));

        assert_eq!(read(&path, None, None).unwrap_err(), ReadError::NeedsPassword);
        assert_eq!(read(&path, Some("nope"), None).unwrap_err(), ReadError::WrongPassword);
        let backup = read(&path, Some("pw"), None).unwrap();
        assert_eq!(backup.notes["abc1"], "the secret note");
        assert_eq!(backup.session.tabs[0].title.as_deref(), Some("Secret title"));

        // Sealed with the app key: no password needed, but the key.
        let key = Key::random();
        write(&path, "x", &session, &notes, Some(&Seal::AppKey(key.clone()))).unwrap();
        assert!(matches!(read(&path, None, None), Err(ReadError::Failed(_))));
        assert_eq!(read(&path, None, Some(&key)).unwrap().notes["abc1"], "the secret note");
        assert_eq!(read(&path, None, Some(&Key::random())).unwrap_err(), ReadError::WrongPassword);

        // Truncated file.
        std::fs::write(&path, &raw[..raw.len() / 2]).unwrap();
        assert!(read(&path, Some("pw"), None).is_err());
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn rejects_other_archives_and_bad_names() {
        // A tar.gz without manifest
        let path = tmp("other.tar.gz");
        {
            let file = File::create(&path).unwrap();
            let mut tar = tar::Builder::new(GzEncoder::new(file, Compression::default()));
            let mut header = tar::Header::new_gnu();
            header.set_size(2);
            header.set_cksum();
            tar.append_data(&mut header, "notes/../../evil.txt", &b"hi"[..]).ok();
            tar.into_inner().unwrap().finish().unwrap();
        }
        assert!(read(&path, None, None).is_err());
        // Not gzip at all
        std::fs::write(&path, b"plain text").unwrap();
        assert!(read(&path, None, None).is_err());
        std::fs::remove_file(path).ok();

        assert!(valid_id("18e51c437230ea3001"));
        assert!(!valid_id("../x"));
        assert!(!valid_id(""));
    }
}
