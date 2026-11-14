//! Sync targets besides GitHub: WebDAV (also Nextcloud with its browser
//! sign-in), a folder that another program syncs (Dropbox, Syncthing, …) and
//! S3-compatible object storage. They hold the same encrypted files as the
//! GitHub sync, below `quicknotes/` (see `github.rs`, which does the merge).
//!
//! Passwords and secret keys are kept in the keyring like the GitHub token.
//! Network calls block; they run in a worker thread.

use crate::crypto::Key;
use crate::github::{self, secret, Error, Job, Outcome, Remote, Result, Snapshot, DIR, INDEX, VAULT};
use base64::Engine;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// A configured sync target.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Target {
    /// A WebDAV folder (`url` ends with `/`).
    Webdav { url: String, user: String },
    /// A folder of a Nextcloud account, signed in with an app password.
    Nextcloud { server: String, user: String, folder: String },
    /// A local folder that another program syncs.
    Folder { path: PathBuf },
    /// A bucket of S3-compatible storage (`endpoint` empty: AWS).
    S3 { endpoint: String, region: String, bucket: String, prefix: String, access_key: String },
}

impl Target {
    /// Identifies the target (keyring entries, sync state).
    pub fn id(&self) -> String {
        match self {
            Target::Webdav { url, user } => format!("webdav:{user}@{url}"),
            Target::Nextcloud { server, user, folder } => format!("nextcloud:{user}@{server}/{folder}"),
            Target::Folder { path } => format!("folder:{}", path.display()),
            Target::S3 { endpoint, bucket, prefix, .. } => format!("s3:{endpoint}/{bucket}/{prefix}"),
        }
    }

    /// The kind, for messages ("Nextcloud: 2 received").
    pub fn name(&self) -> &'static str {
        match self {
            Target::Webdav { .. } => "WebDAV",
            Target::Nextcloud { .. } => "Nextcloud",
            Target::Folder { .. } => t!("Folder"),
            Target::S3 { .. } => "S3",
        }
    }

    /// Where the notes are, for the preferences.
    pub fn describe(&self) -> String {
        match self {
            Target::Webdav { url, user } => format!("{user} – {url}"),
            Target::Nextcloud { server, user, folder } => format!("{user} – {server} ({folder})"),
            Target::Folder { path } => path.display().to_string(),
            Target::S3 { endpoint, bucket, prefix, .. } => {
                let host = if endpoint.is_empty() { "AWS" } else { endpoint.as_str() };
                if prefix.is_empty() { format!("{bucket} – {host}") } else { format!("{bucket}/{prefix} – {host}") }
            }
        }
    }

    /// Keyring entry of the password or secret key.
    pub fn secret_name(&self) -> Option<String> {
        match self {
            Target::Folder { .. } => None,
            _ => Some(format!("secret:{}", self.id())),
        }
    }

    /// The target with the secret from the keyring.
    pub fn open(&self) -> Result<Box<dyn Remote>> {
        let secret = match self.secret_name() {
            Some(name) => Some(secret::load(&name).ok_or_else(|| Error::Auth(t!("The password is missing – connect again").into()))?),
            None => None,
        };
        self.open_with(secret.as_deref().unwrap_or_default())
    }

    pub fn open_with(&self, secret: &str) -> Result<Box<dyn Remote>> {
        Ok(match self {
            Target::Webdav { url, user } => Box::new(Dav::new(url, user, secret)),
            Target::Nextcloud { server, user, folder } => Box::new(Dav::new(&nextcloud_url(server, user, folder), user, secret)),
            Target::Folder { path } => Box::new(FolderRemote { root: path.clone() }),
            Target::S3 { endpoint, region, bucket, prefix, access_key } => {
                Box::new(S3::new(endpoint, region, bucket, prefix, access_key, secret)?)
            }
        })
    }
}

/// Checks that the target is reachable and usable and creates its folder if
/// needed; the wrapped key (`None`: not set up yet).
pub fn prepare(target: &Target, secret: &str) -> Result<Option<crate::crypto::Lock>> {
    let remote = target.open_with(secret)?;
    match target {
        Target::Webdav { url, user } => Dav::new(url, user, secret).ensure_base()?,
        Target::Nextcloud { server, user, folder } => {
            let dav = Dav::new(&format!("{}/remote.php/dav/files/{}/", server.trim_end_matches('/'), enc(user)), user, secret);
            let mut path = String::new();
            for part in folder.split('/').filter(|p| !p.is_empty()) {
                path.push_str(part);
                path.push('/');
                dav.mkcol(&path)?;
            }
        }
        Target::Folder { path } => {
            if !path.is_dir() {
                return Err(Error::Repo(t!("The folder does not exist").into()));
            }
        }
        Target::S3 { .. } => {}
    }
    github::read_vault(&*remote)
}

/// One sync with a target (blocking); secret and key come from the keyring.
pub fn sync(target: &Target, job: Job) -> Result<Outcome> {
    let remote = target.open()?;
    let key = secret::load(&secret::key_name(&target.id())).and_then(|k| Key::import(&k).ok()).ok_or(Error::Key)?;
    github::run(&*remote, &key, job, &mut crate::store::new_id)
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout(Duration::from_secs(120))
        .user_agent("QuickNotes")
        .build()
}

/// Percent-encodes a path segment.
fn enc(segment: &str) -> String {
    urlencoding::encode(segment).into_owned()
}

fn enc_path(path: &str) -> String {
    path.split('/').map(enc).collect::<Vec<_>>().join("/")
}

/// Error of a response: the status and a short text.
fn http_error(e: ureq::Error, what: &str) -> Error {
    match e {
        ureq::Error::Status(401, _) => Error::Auth(t!("Wrong user name or password").into()),
        ureq::Error::Status(403, r) => {
            let body = r.into_string().unwrap_or_default();
            let detail = xml_text(&body, "Message").unwrap_or_default();
            Error::Auth(if detail.is_empty() { t!("Access denied").into() } else { detail })
        }
        ureq::Error::Status(404, _) => Error::Repo(t!("{} not found", what)),
        ureq::Error::Status(507, _) => Error::Net(t!("The storage is full").into()),
        ureq::Error::Status(code, r) => {
            let body = r.into_string().unwrap_or_default();
            let detail = xml_text(&body, "Message").or_else(|| xml_text(&body, "message")).unwrap_or_default();
            Error::Net(format!("HTTP {code} {detail}").trim().to_owned())
        }
        ureq::Error::Transport(t) => Error::Net(t.to_string()),
    }
}

/// The text of the first element called `name` (any namespace).
fn xml_text(xml: &str, name: &str) -> Option<String> {
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut inside = false;
    loop {
        match reader.read_event().ok()? {
            quick_xml::events::Event::Start(e) => inside = e.local_name().as_ref() == name.as_bytes(),
            quick_xml::events::Event::Text(t) if inside => return t.unescape().ok().map(|s| s.into_owned()),
            quick_xml::events::Event::Eof => return None,
            _ => {}
        }
    }
}

/// Whether a path belongs to the synced files.
fn known(rel: &str) -> bool {
    rel == VAULT || rel == INDEX || (rel.starts_with(&format!("{DIR}notes/")) && rel.ends_with(".enc") && !rel.contains(' '))
}

// ===== synced folder =========================================================

/// A local folder: `quicknotes/` in it.
pub struct FolderRemote {
    root: PathBuf,
}

impl FolderRemote {
    fn version(path: &Path) -> Option<String> {
        let meta = std::fs::metadata(path).ok()?;
        let modified = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_nanos();
        Some(format!("{}-{modified}", meta.len()))
    }

    fn write(&self, rel: &str, content: &str) -> Result<()> {
        let path = self.root.join(rel);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let tmp = path.with_file_name(format!(".{name}.tmp"));
        std::fs::write(&tmp, content)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }
}

impl Remote for FolderRemote {
    fn snapshot(&self) -> Result<Snapshot> {
        if !self.root.is_dir() {
            return Err(Error::Repo(t!("The folder does not exist").into()));
        }
        let mut files = HashMap::new();
        for rel in [VAULT, INDEX] {
            if let Some(v) = Self::version(&self.root.join(rel)) {
                files.insert(rel.to_owned(), v);
            }
        }
        if let Ok(entries) = std::fs::read_dir(self.root.join(DIR).join("notes")) {
            for entry in entries.flatten() {
                let rel = format!("{DIR}notes/{}", entry.file_name().to_string_lossy());
                if known(&rel) {
                    if let Some(v) = Self::version(&entry.path()) {
                        files.insert(rel, v);
                    }
                }
            }
        }
        Ok(Snapshot { version: files.get(INDEX).cloned().unwrap_or_default(), files })
    }

    fn read(&self, path: &str, _version: &str) -> Result<String> {
        Ok(std::fs::read_to_string(self.root.join(path))?)
    }

    fn commit(&self, snapshot: &Snapshot, writes: &[(String, Option<String>)]) -> Result<()> {
        for (rel, content) in writes.iter().filter(|(r, c)| c.is_some() && r != INDEX) {
            self.write(rel, content.as_deref().unwrap_or_default())?;
        }
        if let Some((_, Some(index))) = writes.iter().find(|(r, _)| r == INDEX) {
            if Self::version(&self.root.join(INDEX)) != snapshot.files.get(INDEX).cloned() {
                return Err(Error::Moved);
            }
            self.write(INDEX, index)?;
        }
        for (rel, _) in writes.iter().filter(|(_, c)| c.is_none()) {
            match std::fs::remove_file(self.root.join(rel)) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.into()),
                _ => {}
            }
        }
        Ok(())
    }

    fn content_addressed(&self) -> bool {
        true
    }
}

// ===== WebDAV ================================================================

/// The WebDAV folder of a Nextcloud account.
pub fn nextcloud_url(server: &str, user: &str, folder: &str) -> String {
    let folder = folder.trim_matches('/');
    let mut url = format!("{}/remote.php/dav/files/{}/", server.trim_end_matches('/'), enc(user));
    if !folder.is_empty() {
        url.push_str(&enc_path(folder));
        url.push('/');
    }
    url
}

pub struct Dav {
    agent: ureq::Agent,
    /// Ends with `/`.
    base: String,
    /// Path of `base` (decoded), to match the `href`s of listings.
    base_path: String,
    auth: String,
}

#[derive(Debug, Default, PartialEq)]
struct DavEntry {
    href: String,
    etag: String,
    modified: String,
    size: String,
    dir: bool,
}

/// Responses of a `PROPFIND`.
fn parse_multistatus(xml: &str) -> Vec<DavEntry> {
    use quick_xml::events::Event;
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut out = Vec::new();
    let mut current: Option<DavEntry> = None;
    let mut field = String::new();
    while let Ok(event) = reader.read_event() {
        match event {
            Event::Start(e) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                if name == "response" {
                    current = Some(DavEntry::default());
                } else if name == "collection" {
                    if let Some(c) = current.as_mut() {
                        c.dir = true;
                    }
                }
                field = name;
            }
            Event::Empty(e) => {
                if e.local_name().as_ref() == b"collection" {
                    if let Some(c) = current.as_mut() {
                        c.dir = true;
                    }
                }
            }
            Event::Text(t) => {
                let text = t.unescape().map(|s| s.trim().to_owned()).unwrap_or_default();
                if let Some(c) = current.as_mut() {
                    match field.as_str() {
                        "href" => c.href.push_str(&text),
                        "getetag" => c.etag = text,
                        "getlastmodified" => c.modified = text,
                        "getcontentlength" => c.size = text,
                        _ => {}
                    }
                }
            }
            Event::End(e) => {
                if e.local_name().as_ref() == b"response" {
                    out.extend(current.take());
                }
                field.clear();
            }
            Event::Eof => break,
            _ => {}
        }
    }
    out
}

/// An ETag (usable for `If-Match`) rather than a date.
fn is_etag(version: &str) -> bool {
    version.starts_with('"') || version.starts_with("W/")
}

const PROPFIND: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<d:propfind xmlns:d="DAV:"><d:prop><d:getetag/><d:getlastmodified/><d:getcontentlength/><d:resourcetype/></d:prop></d:propfind>"#;

impl Dav {
    pub fn new(url: &str, user: &str, password: &str) -> Self {
        let mut base = url.trim().to_owned();
        if !base.ends_with('/') {
            base.push('/');
        }
        let path_part = base.split_once("://").and_then(|(_, rest)| rest.find('/').map(|i| rest[i..].to_owned())).unwrap_or("/".into());
        let base_path = urlencoding::decode(&path_part).map(|p| p.into_owned()).unwrap_or(path_part);
        let auth = format!("Basic {}", base64::engine::general_purpose::STANDARD.encode(format!("{user}:{password}")));
        Dav { agent: agent(), base, base_path, auth }
    }

    fn url(&self, rel: &str) -> String {
        format!("{}{}", self.base, enc_path(rel))
    }

    fn request(&self, method: &str, rel: &str) -> ureq::Request {
        self.agent.request(method, &self.url(rel)).set("Authorization", &self.auth)
    }

    /// The entries of a folder (`None`: it does not exist).
    fn list(&self, rel: &str) -> Result<Option<Vec<(String, DavEntry)>>> {
        let response = match self.request("PROPFIND", rel).set("Depth", "1").set("Content-Type", "application/xml").send_string(PROPFIND) {
            Ok(r) => r,
            Err(ureq::Error::Status(404, _)) => return Ok(None),
            Err(e) => return Err(http_error(e, t!("Folder"))),
        };
        let body = response.into_string()?;
        let mut out = Vec::new();
        for entry in parse_multistatus(&body) {
            let href = match entry.href.split_once("://") {
                Some((_, rest)) => rest.find('/').map(|i| rest[i..].to_owned()).unwrap_or_default(),
                None => entry.href.clone(),
            };
            let decoded = urlencoding::decode(&href).map(|p| p.into_owned()).unwrap_or(href);
            if let Some(rel) = decoded.strip_prefix(&self.base_path) {
                out.push((rel.trim_end_matches('/').to_owned(), entry));
            }
        }
        Ok(Some(out))
    }

    fn mkcol(&self, rel: &str) -> Result<()> {
        match self.request("MKCOL", rel).call() {
            // 405: exists already.
            Ok(_) | Err(ureq::Error::Status(405, _)) => Ok(()),
            Err(e) => Err(http_error(e, t!("Folder"))),
        }
    }

    /// The base folder exists (it is created if its parent does).
    fn ensure_base(&self) -> Result<()> {
        match self.agent.request("PROPFIND", &self.base).set("Authorization", &self.auth).set("Depth", "0").call() {
            Ok(_) => Ok(()),
            Err(ureq::Error::Status(404, _)) => match self.agent.request("MKCOL", &self.base).set("Authorization", &self.auth).call() {
                Ok(_) => Ok(()),
                Err(e) => Err(http_error(e, t!("Folder"))),
            },
            Err(e) => Err(http_error(e, t!("Folder"))),
        }
    }

    #[allow(clippy::result_large_err)]
    fn put(&self, rel: &str, content: &str, condition: Option<(&str, &str)>) -> std::result::Result<(), ureq::Error> {
        let mut request = self.request("PUT", rel).set("Content-Type", "application/octet-stream");
        if let Some((name, value)) = condition {
            request = request.set(name, value);
        }
        request.send_string(content).map(|_| ())
    }
}

impl Remote for Dav {
    fn snapshot(&self) -> Result<Snapshot> {
        let mut files = HashMap::new();
        for dir in [DIR.to_owned(), format!("{DIR}notes/")] {
            let Some(entries) = self.list(&dir)? else { continue };
            for (rel, e) in entries {
                if e.dir || !known(&rel) {
                    continue;
                }
                let version = if e.etag.is_empty() { format!("{}-{}", e.size, e.modified) } else { e.etag };
                files.insert(rel, version);
            }
        }
        Ok(Snapshot { version: files.get(INDEX).cloned().unwrap_or_default(), files })
    }

    fn read(&self, path: &str, _version: &str) -> Result<String> {
        self.request("GET", path).call().map_err(|e| http_error(e, path))?.into_string().map_err(Into::into)
    }

    fn commit(&self, snapshot: &Snapshot, writes: &[(String, Option<String>)]) -> Result<()> {
        if !snapshot.files.contains_key(VAULT) {
            self.mkcol(DIR)?;
        }
        let notes_dir = format!("{DIR}notes/");
        let has_notes = snapshot.files.keys().any(|p| p.starts_with(&notes_dir));
        if !has_notes && writes.iter().any(|(p, c)| c.is_some() && p.starts_with(&notes_dir)) {
            self.mkcol(&notes_dir)?;
        }
        for (rel, content) in writes.iter().filter(|(r, c)| c.is_some() && r != INDEX) {
            let condition = (rel == VAULT).then_some(("If-None-Match", "*"));
            match self.put(rel, content.as_deref().unwrap_or_default(), condition) {
                Ok(()) => {}
                Err(ureq::Error::Status(412, _)) => return Err(Error::Moved),
                Err(e) => return Err(http_error(e, rel)),
            }
        }
        if let Some((_, Some(index))) = writes.iter().find(|(r, _)| r == INDEX) {
            let condition = match snapshot.files.get(INDEX) {
                Some(v) if is_etag(v) => Some(("If-Match", v.as_str())),
                Some(_) => None,
                None => Some(("If-None-Match", "*")),
            };
            match self.put(INDEX, index, condition) {
                Ok(()) => {}
                Err(ureq::Error::Status(412, _)) => return Err(Error::Moved),
                Err(e) => return Err(http_error(e, INDEX)),
            }
        }
        for (rel, _) in writes.iter().filter(|(_, c)| c.is_none()) {
            match self.request("DELETE", rel).call() {
                Ok(_) | Err(ureq::Error::Status(404, _)) => {}
                Err(e) => return Err(http_error(e, rel)),
            }
        }
        Ok(())
    }

    fn content_addressed(&self) -> bool {
        true
    }
}

// ===== Nextcloud sign-in ======================================================

/// A running Nextcloud sign-in (Login Flow v2): the user confirms in the
/// browser, QuickNotes polls for the app password.
pub struct LoginFlow {
    pub login_url: String,
    token: String,
    endpoint: String,
}

/// `https://` is added when missing; no trailing slash.
pub fn normalize_server(input: &str) -> String {
    let s = input.trim().trim_end_matches('/');
    let s = s.strip_suffix("/index.php").unwrap_or(s);
    if s.contains("://") { s.to_owned() } else { format!("https://{s}") }
}

pub fn nextcloud_start(server: &str) -> Result<LoginFlow> {
    let response = agent()
        .post(&format!("{server}/index.php/login/v2"))
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(404, _) => Error::Repo(t!("No Nextcloud found at this address").into()),
            e => http_error(e, "Nextcloud"),
        })?;
    let v: serde_json::Value = response.into_json()?;
    let get = |p: &str| v.pointer(p).and_then(|x| x.as_str()).map(str::to_owned);
    match (get("/login"), get("/poll/token"), get("/poll/endpoint")) {
        (Some(login_url), Some(token), Some(endpoint)) => Ok(LoginFlow { login_url, token, endpoint }),
        _ => Err(Error::Repo(t!("No Nextcloud found at this address").into())),
    }
}

/// Waits until the sign-in is confirmed: (server, user, app password).
pub fn nextcloud_finish(flow: &LoginFlow, cancel: Arc<AtomicBool>) -> Result<(String, String, String)> {
    let agent = agent();
    for _ in 0..600 {
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        match agent.post(&flow.endpoint).send_form(&[("token", &flow.token)]) {
            Ok(r) => {
                let v: serde_json::Value = r.into_json()?;
                let get = |k: &str| v[k].as_str().unwrap_or_default().to_owned();
                return Ok((get("server"), get("loginName"), get("appPassword")));
            }
            // Not confirmed yet.
            Err(ureq::Error::Status(404, _)) => {}
            Err(e) => return Err(http_error(e, "Nextcloud")),
        }
        std::thread::sleep(Duration::from_secs(2));
    }
    Err(Error::Cancelled)
}

// ===== S3 ======================================================================

type HmacSha256 = Hmac<Sha256>;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn sha256_hex(data: &[u8]) -> String {
    hex(&Sha256::digest(data))
}

fn hmac(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("any key length");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// URI encoding of AWS Signature V4 (`/` kept in paths).
fn aws_encode(s: &str, keep_slash: bool) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b'/' if keep_slash => out.push('/'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// The `Authorization` header (AWS Signature Version 4). `headers` are the
/// signed headers with lower-case names; `query` is encoded already.
#[allow(clippy::too_many_arguments)]
fn sign_v4(
    method: &str,
    uri: &str,
    query: &[(String, String)],
    headers: &[(String, String)],
    payload_hash: &str,
    amz_date: &str,
    region: &str,
    access_key: &str,
    secret_key: &str,
) -> String {
    let mut query = query.to_vec();
    query.sort();
    let canonical_query = query.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("&");
    let mut headers = headers.to_vec();
    headers.sort();
    let canonical_headers: String = headers.iter().map(|(k, v)| format!("{k}:{}\n", v.trim())).collect();
    let signed = headers.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>().join(";");
    let canonical = format!("{method}\n{uri}\n{canonical_query}\n{canonical_headers}\n{signed}\n{payload_hash}");
    let date = &amz_date[..8];
    let scope = format!("{date}/{region}/s3/aws4_request");
    let to_sign = format!("AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}", sha256_hex(canonical.as_bytes()));
    let k_date = hmac(format!("AWS4{secret_key}").as_bytes(), date.as_bytes());
    let k_region = hmac(&k_date, region.as_bytes());
    let k_service = hmac(&k_region, b"s3");
    let k_signing = hmac(&k_service, b"aws4_request");
    let signature = hex(&hmac(&k_signing, to_sign.as_bytes()));
    format!("AWS4-HMAC-SHA256 Credential={access_key}/{scope}, SignedHeaders={signed}, Signature={signature}")
}

pub struct S3 {
    agent: ureq::Agent,
    /// `scheme://host[:port]`
    origin: String,
    host: String,
    /// `/bucket` for path-style addressing, else empty.
    bucket_path: String,
    region: String,
    access_key: String,
    secret_key: String,
    /// `prefix/` or empty.
    prefix: String,
}

impl S3 {
    pub fn new(endpoint: &str, region: &str, bucket: &str, prefix: &str, access_key: &str, secret_key: &str) -> Result<Self> {
        let region = if region.trim().is_empty() { "us-east-1".to_owned() } else { region.trim().to_owned() };
        let bucket = bucket.trim();
        if bucket.is_empty() {
            return Err(Error::Repo(t!("Enter a bucket").into()));
        }
        let endpoint = endpoint.trim().trim_end_matches('/');
        let (scheme, host) = match endpoint.split_once("://") {
            Some((s, h)) => (s.to_owned(), h.to_owned()),
            None if endpoint.is_empty() => ("https".to_owned(), format!("s3.{region}.amazonaws.com")),
            None => ("https".to_owned(), endpoint.to_owned()),
        };
        // AWS: the bucket in the host name; others: in the path.
        let (host, bucket_path) =
            if host.ends_with("amazonaws.com") { (format!("{bucket}.{host}"), String::new()) } else { (host, format!("/{}", aws_encode(bucket, false))) };
        let prefix = prefix.trim().trim_matches('/');
        Ok(S3 {
            agent: agent(),
            origin: format!("{scheme}://{host}"),
            host,
            bucket_path,
            region,
            access_key: access_key.trim().to_owned(),
            secret_key: secret_key.to_owned(),
            prefix: if prefix.is_empty() { String::new() } else { format!("{prefix}/") },
        })
    }

    #[allow(clippy::result_large_err)]
    fn call(
        &self,
        method: &str,
        key: Option<&str>,
        query: &[(&str, &str)],
        body: &[u8],
        extra: &[(&str, &str)],
    ) -> std::result::Result<ureq::Response, ureq::Error> {
        let uri = match key {
            Some(k) => format!("{}/{}", self.bucket_path, aws_encode(&format!("{}{k}", self.prefix), true)),
            None => format!("{}/", self.bucket_path),
        };
        let query: Vec<(String, String)> = query.iter().map(|(k, v)| (aws_encode(k, false), aws_encode(v, false))).collect();
        let amz_date = jiff::Timestamp::now().strftime("%Y%m%dT%H%M%SZ").to_string();
        let payload = sha256_hex(body);
        let headers = vec![
            ("host".to_owned(), self.host.clone()),
            ("x-amz-content-sha256".to_owned(), payload.clone()),
            ("x-amz-date".to_owned(), amz_date.clone()),
        ];
        let auth = sign_v4(method, &uri, &query, &headers, &payload, &amz_date, &self.region, &self.access_key, &self.secret_key);
        let mut url = format!("{}{uri}", self.origin);
        if !query.is_empty() {
            let mut sorted = query.clone();
            sorted.sort();
            url.push('?');
            url.push_str(&sorted.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("&"));
        }
        let mut request = self
            .agent
            .request(method, &url)
            .set("x-amz-content-sha256", &payload)
            .set("x-amz-date", &amz_date)
            .set("Authorization", &auth);
        for (name, value) in extra {
            request = request.set(name, value);
        }
        if method == "PUT" {
            request.send_bytes(body)
        } else {
            request.call()
        }
    }

    fn put(&self, rel: &str, content: &str, condition: Option<(&str, &str)>) -> Result<()> {
        let extra: Vec<(&str, &str)> = condition.into_iter().collect();
        match self.call("PUT", Some(rel), &[], content.as_bytes(), &extra) {
            Ok(_) => Ok(()),
            Err(ureq::Error::Status(412, _)) => Err(Error::Moved),
            // Conditional writes not supported: write anyway (the revisions
            // of the index catch a lost write).
            Err(ureq::Error::Status(501, _)) if condition.is_some() => self.put(rel, content, None),
            Err(e) => Err(http_error(e, rel)),
        }
    }
}

/// `Contents` of a `ListObjectsV2` response: (key, ETag), whether there is
/// more and the continuation token.
fn parse_list(xml: &str) -> (Vec<(String, String)>, bool, String) {
    use quick_xml::events::Event;
    let mut reader = quick_xml::Reader::from_str(xml);
    let (mut items, mut truncated, mut token) = (Vec::new(), false, String::new());
    let (mut key, mut etag, mut field) = (String::new(), String::new(), String::new());
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                field = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                if field == "Contents" {
                    key.clear();
                    etag.clear();
                }
            }
            Ok(Event::Text(t)) => {
                let text = t.unescape().map(|s| s.into_owned()).unwrap_or_default();
                match field.as_str() {
                    "Key" => key.push_str(&text),
                    "ETag" => etag.push_str(&text),
                    "IsTruncated" => truncated = text.trim() == "true",
                    "NextContinuationToken" => token = text,
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                if e.local_name().as_ref() == b"Contents" {
                    items.push((key.clone(), etag.clone()));
                }
                field.clear();
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    (items, truncated, token)
}

impl Remote for S3 {
    fn snapshot(&self) -> Result<Snapshot> {
        let mut files = HashMap::new();
        let list_prefix = format!("{}{DIR}", self.prefix);
        let mut token = String::new();
        loop {
            let mut query = vec![("list-type", "2"), ("prefix", list_prefix.as_str())];
            if !token.is_empty() {
                query.push(("continuation-token", token.as_str()));
            }
            let body = self.call("GET", None, &query, b"", &[]).map_err(|e| http_error(e, t!("Bucket")))?.into_string()?;
            let (items, truncated, next) = parse_list(&body);
            for (key, etag) in items {
                if let Some(rel) = key.strip_prefix(&self.prefix) {
                    if known(rel) {
                        files.insert(rel.to_owned(), etag);
                    }
                }
            }
            if !truncated || next.is_empty() {
                break;
            }
            token = next;
        }
        Ok(Snapshot { version: files.get(INDEX).cloned().unwrap_or_default(), files })
    }

    fn read(&self, path: &str, _version: &str) -> Result<String> {
        self.call("GET", Some(path), &[], b"", &[]).map_err(|e| http_error(e, path))?.into_string().map_err(Into::into)
    }

    fn commit(&self, snapshot: &Snapshot, writes: &[(String, Option<String>)]) -> Result<()> {
        for (rel, content) in writes.iter().filter(|(r, c)| c.is_some() && r != INDEX) {
            let condition = (rel == VAULT).then_some(("If-None-Match", "*"));
            self.put(rel, content.as_deref().unwrap_or_default(), condition)?;
        }
        if let Some((_, Some(index))) = writes.iter().find(|(r, _)| r == INDEX) {
            let condition = match snapshot.files.get(INDEX) {
                Some(v) => ("If-Match", v.as_str()),
                None => ("If-None-Match", "*"),
            };
            self.put(INDEX, index, Some(condition))?;
        }
        for (rel, _) in writes.iter().filter(|(_, c)| c.is_none()) {
            match self.call("DELETE", Some(rel), &[], b"", &[]) {
                Ok(_) | Err(ureq::Error::Status(404, _)) => {}
                Err(e) => return Err(http_error(e, rel)),
            }
        }
        Ok(())
    }

    fn content_addressed(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::Lock;
    use crate::github::tests::{folder, note, Machine};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::Mutex;

    #[test]
    fn aws_signature_example() {
        // "Example: GET Object" of the AWS Signature V4 documentation.
        let empty = sha256_hex(b"");
        let headers = vec![
            ("host".to_owned(), "examplebucket.s3.amazonaws.com".to_owned()),
            ("range".to_owned(), "bytes=0-9".to_owned()),
            ("x-amz-content-sha256".to_owned(), empty.clone()),
            ("x-amz-date".to_owned(), "20130524T000000Z".to_owned()),
        ];
        let auth = sign_v4(
            "GET",
            "/test.txt",
            &[],
            &headers,
            &empty,
            "20130524T000000Z",
            "us-east-1",
            "AKIAIOSFODNN7EXAMPLE",
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
        );
        assert!(auth.ends_with("Signature=f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41"), "{auth}");
        assert!(auth.contains("SignedHeaders=host;range;x-amz-content-sha256;x-amz-date"));
    }

    #[test]
    fn parses_listings() {
        let xml = r#"<?xml version="1.0"?><d:multistatus xmlns:d="DAV:"><d:response><d:href>/dav/a%20b/quicknotes/</d:href><d:propstat><d:prop><d:resourcetype><d:collection/></d:resourcetype></d:prop></d:propstat></d:response><d:response><d:href>/dav/a%20b/quicknotes/index.enc</d:href><d:propstat><d:prop><d:getetag>"abc"</d:getetag><d:resourcetype/></d:prop></d:propstat></d:response></d:multistatus>"#;
        let entries = parse_multistatus(xml);
        assert_eq!(entries.len(), 2);
        assert!(entries[0].dir);
        assert_eq!(entries[1].etag, "\"abc\"");
        let xml = "<ListBucketResult><IsTruncated>true</IsTruncated><Contents><Key>p/quicknotes/vault.json</Key><ETag>&quot;e1&quot;</ETag></Contents><NextContinuationToken>t2</NextContinuationToken></ListBucketResult>";
        assert_eq!(parse_list(xml), (vec![("p/quicknotes/vault.json".to_owned(), "\"e1\"".to_owned())], true, "t2".to_owned()));
        assert_eq!(nextcloud_url("https://cloud.x/", "ann b", "/Quick Notes/"), "https://cloud.x/remote.php/dav/files/ann%20b/Quick%20Notes/");
        assert_eq!(normalize_server("cloud.x/index.php"), "https://cloud.x");
    }

    // ----- a small WebDAV and S3 server ---------------------------------------

    #[derive(Default)]
    struct Store {
        files: HashMap<String, (String, u32)>,
        dirs: std::collections::HashSet<String>,
        counter: u32,
    }

    fn read_request(stream: &mut std::net::TcpStream) -> Option<(String, String, HashMap<String, String>, Vec<u8>)> {
        let mut buf = Vec::new();
        let mut byte = [0u8; 1];
        while !buf.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).ok()?;
            buf.push(byte[0]);
        }
        let head = String::from_utf8_lossy(&buf).to_string();
        let mut lines = head.lines();
        let mut first = lines.next()?.split_whitespace();
        let (method, target) = (first.next()?.to_owned(), first.next()?.to_owned());
        let headers: HashMap<String, String> = lines
            .filter_map(|l| l.split_once(':').map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_owned())))
            .collect();
        let len = headers.get("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
        let mut body = vec![0u8; len];
        stream.read_exact(&mut body).ok()?;
        Some((method, target, headers, body))
    }

    /// Serves WebDAV below `/dav/` and an S3 bucket at `/bucket/`.
    fn mock_server() -> (String, Arc<Mutex<Store>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        let shared: Arc<Mutex<Store>> = Default::default();
        let store = shared.clone();
        store.lock().unwrap().dirs.insert("/dav/".into());
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let Some((method, target, headers, body)) = read_request(&mut stream) else { continue };
                let (path, query) = target.split_once('?').unwrap_or((&target, ""));
                let path = urlencoding::decode(path).unwrap().into_owned();
                let mut s = store.lock().unwrap();
                let etag = |n: u32| format!("\"e{n}\"");
                let condition_fails = |s: &Store, key: &str| {
                    let current = s.files.get(key).map(|(_, n)| etag(*n));
                    match (headers.get("if-match"), headers.get("if-none-match"), &current) {
                        (Some(m), _, Some(c)) => m != c,
                        (Some(_), _, None) => true,
                        (_, Some(_), Some(_)) => true,
                        _ => false,
                    }
                };
                let (status, reply) = match method.as_str() {
                    "PROPFIND" => {
                        let dir = if path.ends_with('/') { path.clone() } else { format!("{path}/") };
                        if !s.dirs.contains(&dir) {
                            (404, String::new())
                        } else {
                            let mut xml = String::from(r#"<?xml version="1.0"?><d:multistatus xmlns:d="DAV:">"#);
                            xml.push_str(&format!("<d:response><d:href>{dir}</d:href><d:propstat><d:prop><d:resourcetype><d:collection/></d:resourcetype></d:prop></d:propstat></d:response>"));
                            for (p, (_, n)) in s.files.iter().filter(|(p, _)| p.starts_with(&dir) && !p[dir.len()..].contains('/')) {
                                xml.push_str(&format!("<d:response><d:href>{}</d:href><d:propstat><d:prop><d:getetag>{}</d:getetag><d:resourcetype/></d:prop></d:propstat></d:response>", urlencoding::encode(p).replace("%2F", "/"), etag(*n)));
                            }
                            xml.push_str("</d:multistatus>");
                            (207, xml)
                        }
                    }
                    "MKCOL" => {
                        let dir = if path.ends_with('/') { path.clone() } else { format!("{path}/") };
                        if s.dirs.insert(dir) { (201, String::new()) } else { (405, String::new()) }
                    }
                    "GET" if query.contains("list-type=2") => {
                        let prefix = query.split('&').find_map(|kv| kv.strip_prefix("prefix=")).map(|p| urlencoding::decode(p).unwrap().into_owned()).unwrap_or_default();
                        let mut xml = String::from("<ListBucketResult><IsTruncated>false</IsTruncated>");
                        for (p, (_, n)) in &s.files {
                            if let Some(key) = p.strip_prefix("/bucket/").filter(|k| k.starts_with(&prefix)) {
                                xml.push_str(&format!("<Contents><Key>{key}</Key><ETag>{}</ETag></Contents>", etag(*n).replace('"', "&quot;")));
                            }
                        }
                        xml.push_str("</ListBucketResult>");
                        (200, xml)
                    }
                    "GET" => match s.files.get(&path) {
                        Some((c, _)) => (200, c.clone()),
                        None => (404, String::new()),
                    },
                    "PUT" => {
                        let parent = format!("{}/", &path[..path.rfind('/').unwrap()]);
                        if path.starts_with("/dav/") && !s.dirs.contains(&parent) {
                            (409, String::new())
                        } else if condition_fails(&s, &path) {
                            (412, String::new())
                        } else {
                            s.counter += 1;
                            let n = s.counter;
                            s.files.insert(path.clone(), (String::from_utf8(body).unwrap(), n));
                            (201, String::new())
                        }
                    }
                    "DELETE" => match s.files.remove(&path) {
                        Some(_) => (204, String::new()),
                        None => (404, String::new()),
                    },
                    _ => (405, String::new()),
                };
                drop(s);
                let _ = write!(stream, "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}", reply.len());
            }
        });
        (base, shared)
    }

    /// Two machines through one target: sending, receiving, conflicts,
    /// deletions, and the cleanup of old files.
    fn two_machines(remote: &dyn Remote, files: impl Fn() -> Vec<String>) {
        assert_eq!(github::read_vault(remote).unwrap(), None);
        let (lock, key) = Lock::create("pw", crate::crypto::tests::fast_kdf());
        github::write_vault(remote, &lock).unwrap();
        let key_b = github::read_vault(remote).unwrap().unwrap().open("pw").unwrap();

        let mut a = Machine::new("a", vec![note("n1", "first"), note("n2", "second")]);
        a.folders = vec![folder("f", "Work", None)];
        let mut b = Machine::new("b", vec![]);
        assert_eq!(a.sync(remote, &key).sent, 2);
        assert_eq!(a.sync(remote, &key).sent, 0);
        assert_eq!(b.sync(remote, &key_b).changes.len(), 2);
        assert_eq!(b.content("n1"), Some("first"));
        assert_eq!(b.folders, [folder("f", "Work", None)]);

        // Edits go across; the old version's file is removed.
        b.edit("n1", "edited on B");
        b.sync(remote, &key_b);
        a.sync(remote, &key);
        assert_eq!(a.content("n1"), Some("edited on B"));
        let note_files = files().into_iter().filter(|f| f.contains("notes/n1-")).count();
        assert_eq!(note_files, 1, "{:?}", files());

        // Both edit: both versions are kept.
        a.edit("n1", "A");
        b.edit("n1", "B");
        b.sync(remote, &key_b);
        assert_eq!(a.sync(remote, &key).conflicts, 1);
        b.sync(remote, &key_b);
        assert!(b.notes.iter().any(|n| n.content == "A") && b.notes.iter().any(|n| n.content == "B"));

        // Deletions.
        a.notes.retain(|n| n.id != "n2");
        a.sync(remote, &key);
        b.sync(remote, &key_b);
        assert!(b.content("n2").is_none());
        assert!(!files().iter().any(|f| f.contains("notes/n2-")), "{:?}", files());

        // Nothing readable is stored.
        assert!(!files().is_empty());
    }

    #[test]
    fn webdav_sync() {
        let (base, store) = mock_server();
        let target = Target::Webdav { url: format!("{base}/dav/My Notes"), user: "u".into() };
        assert!(prepare(&target, "pw").unwrap().is_none(), "the base folder is created");
        let remote = target.open_with("pw").unwrap();
        two_machines(&*remote, || store.lock().unwrap().files.keys().cloned().collect());
        for (path, (content, _)) in &store.lock().unwrap().files {
            assert!(!content.contains("first") && !content.contains("Work"), "{path}");
        }
    }

    #[test]
    fn s3_sync() {
        let (base, store) = mock_server();
        let remote = S3::new(&base, "eu-1", "bucket", "/team/", "AK", "SK").unwrap();
        two_machines(&remote, || store.lock().unwrap().files.keys().cloned().collect());
        assert!(store.lock().unwrap().files.keys().all(|k| k.starts_with("/bucket/team/quicknotes/")));
    }

    /// Against real servers, e.g.
    /// `QN_IT_DAV="https://host/dav/folder/|user|password;…"`,
    /// `QN_IT_NC="https://cloud|user|password"`,
    /// `QN_IT_S3="endpoint|region|bucket|key|secret"`.
    #[test]
    #[ignore]
    fn real_servers() {
        let stamp = jiff::Timestamp::now().as_second();
        let run = |target: Target, secret: &str| {
            eprintln!("{}", target.describe());
            assert_eq!(prepare(&target, secret).unwrap(), None);
            let remote = target.open_with(secret).unwrap();
            let lister = target.open_with(secret).unwrap();
            two_machines(&*remote, || lister.snapshot().unwrap().files.keys().cloned().collect());
        };
        for spec in std::env::var("QN_IT_DAV").unwrap_or_default().split(';').filter(|s| !s.is_empty()) {
            let p: Vec<&str> = spec.split('|').collect();
            run(Target::Webdav { url: format!("{}qn-{stamp}/", p[0]), user: p[1].into() }, p[2]);
        }
        if let Ok(spec) = std::env::var("QN_IT_NC") {
            let p: Vec<&str> = spec.split('|').collect();
            run(Target::Nextcloud { server: p[0].into(), user: p[1].into(), folder: format!("QN {stamp}/Sub") }, p[2]);
            let flow = nextcloud_start(p[0]).unwrap();
            assert!(flow.login_url.contains("/login/v2/flow/"), "{}", flow.login_url);
        }
        if let Ok(spec) = std::env::var("QN_IT_S3") {
            let p: Vec<&str> = spec.split('|').collect();
            let bucket = format!("qn-{stamp}");
            let s3 = S3::new(p[0], p[1], &bucket, "", p[3], p[4]).unwrap();
            s3.call("PUT", None, &[], b"", &[]).unwrap();
            run(Target::S3 { endpoint: p[0].into(), region: p[1].into(), bucket, prefix: "team".into(), access_key: p[3].into() }, p[4]);
        }
    }

    #[test]
    fn folder_sync_and_lost_writes() {
        let dir = std::env::temp_dir().join(format!("qn-sync-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let remote = FolderRemote { root: dir.clone() };
        let list = || -> Vec<String> {
            let mut out = Vec::new();
            for sub in ["quicknotes", "quicknotes/notes"] {
                for e in std::fs::read_dir(dir.join(sub)).into_iter().flatten().flatten() {
                    out.push(format!("{sub}/{}", e.file_name().to_string_lossy()));
                }
            }
            out
        };
        two_machines(&remote, list);

        // A write of A gets overwritten by an older index (two machines at
        // once on a synced folder): A notices and keeps its version.
        let key = github::read_vault(&remote).unwrap().unwrap().open("pw").unwrap();
        let mut a = Machine::new("c", vec![note("x", "one")]);
        a.sync(&remote, &key);
        let old = std::fs::read(dir.join(INDEX)).unwrap();
        let old_files: Vec<(PathBuf, Vec<u8>)> = std::fs::read_dir(dir.join("quicknotes/notes"))
            .unwrap()
            .flatten()
            .map(|e| (e.path(), std::fs::read(e.path()).unwrap()))
            .collect();
        a.edit("x", "two");
        a.sync(&remote, &key);
        std::fs::write(dir.join(INDEX), old).unwrap();
        for (p, c) in old_files {
            std::fs::write(p, c).unwrap();
        }
        let out = a.sync(&remote, &key);
        assert_eq!(a.content("x"), Some("two"), "local version kept");
        assert_eq!(out.conflicts, 1, "the other version comes as a copy");
        let mut c = Machine::new("d", vec![]);
        c.sync(&remote, &key);
        assert!(c.notes.iter().any(|n| n.content == "two"), "sent again");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
