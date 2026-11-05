//! Backup archives (`.tar.gz`) of all notes and the session.
//!
//! Layout:
//! - `manifest.json`  format version, app version, creation time
//! - `session.json`   tabs and settings (same format as the live session)
//! - `notes/<id>.txt` contents of notes and of unsaved changes to opened files
//!
//! Archives are only read into memory, never extracted to disk, so entry
//! names cannot write anywhere. Unknown entries are ignored.

use crate::store::Session;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::Path;

const FORMAT: u32 = 1;
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
}

/// Writes a backup archive to `path`.
pub fn write(path: &Path, created: &str, session: &Session, notes: &[(String, String)]) -> Result<(), String> {
    let err = |e: std::io::Error| format!("{}: {e}", path.display());
    let file = File::create(path).map_err(err)?;
    let mut tar = tar::Builder::new(GzEncoder::new(BufWriter::new(file), Compression::default()));
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
    add("manifest.json", json(&manifest).as_bytes()).map_err(err)?;
    add("session.json", json(session).as_bytes()).map_err(err)?;
    for (id, content) in notes {
        if valid_id(id) {
            add(&format!("notes/{id}.txt"), content.as_bytes()).map_err(err)?;
        }
    }
    let gz = tar.into_inner().map_err(err)?;
    let mut out = gz.finish().map_err(err)?;
    out.flush().map_err(err)?;
    Ok(())
}

/// Reads and validates a backup archive.
pub fn read(path: &Path) -> Result<Backup, String> {
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut archive = tar::Archive::new(GzDecoder::new(file));
    let not_backup = || t!("This is not a QuickNotes backup (.tar.gz)").to_owned();
    let mut manifest = None;
    let mut session = None;
    let mut notes = HashMap::new();
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
            return Err(t!("The backup is unexpectedly large").into());
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
            n => {
                if let Some(id) = n.strip_prefix("notes/").and_then(|f| f.strip_suffix(".txt")) {
                    if valid_id(id) {
                        notes.insert(id.to_owned(), text());
                    }
                }
            }
        }
    }

    let manifest = manifest.ok_or_else(not_backup)?;
    if manifest.format > FORMAT {
        return Err(t!("The backup was created by a newer QuickNotes version").into());
    }
    let session = session.ok_or_else(not_backup)?;
    Ok(Backup { manifest, session, notes })
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
        let notes = vec![("abc1".to_owned(), "hello\nwörld".to_owned())];
        write(&path, "2026-11-05T12:00:00", &session, &notes).unwrap();

        let backup = read(&path).unwrap();
        assert_eq!(backup.manifest.format, FORMAT);
        assert_eq!(backup.manifest.created, "2026-11-05T12:00:00");
        assert_eq!(backup.session.tabs[0].id, "abc1");
        assert_eq!(backup.session.settings.font_family, "Fira Code");
        assert_eq!(backup.notes["abc1"], "hello\nwörld");
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
        assert!(read(&path).is_err());
        // Not gzip at all
        std::fs::write(&path, b"plain text").unwrap();
        assert!(read(&path).is_err());
        std::fs::remove_file(path).ok();

        assert!(valid_id("18e51c437230ea3001"));
        assert!(!valid_id("../x"));
        assert!(!valid_id(""));
    }
}
