//! Google Drive sync.
//!
//! - OAuth 2.0 for installed apps: the browser redirects to a loopback port
//!   (`http://127.0.0.1:<port>`), secured with PKCE and a `state` value. The
//!   user brings their own OAuth client (Google Cloud project).
//! - Scope `drive.file`: the app only sees the files it created itself.
//! - All notes live in the folder `QuickNotes`, one file per tab. The tab id is
//!   kept in the file's `appProperties`, so files can be renamed freely.
//! - The client secret and the refresh token are kept in the Secret Service
//!   (GNOME keyring), never in the settings.
//!
//! Network calls block; they run in a worker thread (see `window.rs`).

use base64::Engine;
use gtk::gio;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const REVOKE_URL: &str = "https://oauth2.googleapis.com/revoke";
const FILES_URL: &str = "https://www.googleapis.com/drive/v3/files";
const UPLOAD_URL: &str = "https://www.googleapis.com/upload/drive/v3/files";
const ABOUT_URL: &str = "https://www.googleapis.com/drive/v3/about";
const SCOPE: &str = "https://www.googleapis.com/auth/drive.file";
pub const FOLDER_NAME: &str = "QuickNotes";
const FOLDER_MIME: &str = "application/vnd.google-apps.folder";
/// How long the browser sign-in may take.
const AUTH_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Debug)]
pub enum Error {
    /// The authorisation is missing, expired or was revoked: connect again.
    Auth(String),
    Net(String),
    Cancelled,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Auth(e) => write!(f, "{}: {e}", t!("Google Drive authorisation failed")),
            Error::Net(e) => write!(f, "{e}"),
            Error::Cancelled => write!(f, "{}", t!("Cancelled")),
        }
    }
}

impl From<ureq::Error> for Error {
    fn from(e: ureq::Error) -> Self {
        match e {
            ureq::Error::Status(code, resp) => {
                let body = resp.into_string().unwrap_or_default();
                let short: String = body.chars().take(300).collect();
                if code == 401 || body.contains("invalid_grant") || body.contains("invalid_client") {
                    Error::Auth(short)
                } else {
                    Error::Net(format!("HTTP {code}: {short}"))
                }
            }
            ureq::Error::Transport(t) => Error::Net(t.to_string()),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Net(e.to_string())
    }
}

type Result<T> = std::result::Result<T, Error>;

// ===== secrets ==============================================================

/// Values in the Secret Service: `client-secret` and `refresh-token`.
pub mod secret {
    use super::gio;
    use std::collections::HashMap;

    pub const CLIENT_SECRET: &str = "client-secret";
    pub const REFRESH_TOKEN: &str = "refresh-token";

    fn schema() -> libsecret::Schema {
        let attributes = HashMap::from([("kind", libsecret::SchemaAttributeType::String)]);
        libsecret::Schema::new("io.github.quicknotes.GoogleDrive", libsecret::SchemaFlags::NONE, attributes)
    }

    pub fn store(kind: &str, value: &str) -> Result<(), String> {
        let label = format!("QuickNotes – Google Drive ({kind})");
        libsecret::password_store_sync(
            Some(&schema()),
            HashMap::from([("kind", kind)]),
            Some(libsecret::COLLECTION_DEFAULT.as_str()),
            &label,
            value,
            None::<&gio::Cancellable>,
        )
        .map_err(|e| e.to_string())
    }

    pub fn load(kind: &str) -> Option<String> {
        libsecret::password_lookup_sync(Some(&schema()), HashMap::from([("kind", kind)]), None::<&gio::Cancellable>)
            .ok()
            .flatten()
            .map(|s| s.to_string())
    }

    pub fn clear(kind: &str) {
        let _ = libsecret::password_clear_sync(Some(&schema()), HashMap::from([("kind", kind)]), None::<&gio::Cancellable>);
    }
}

// ===== OAuth ================================================================

fn random_token(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    getrandom::getrandom(&mut buf).expect("random numbers");
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(buf)
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout(Duration::from_secs(120))
        .build()
}

/// A started sign-in: open `url` in the browser, then call [`finish_auth`].
pub struct AuthRequest {
    listener: TcpListener,
    verifier: String,
    state: String,
    redirect: String,
    pub url: String,
}

pub fn begin_auth(client_id: &str) -> Result<AuthRequest> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let redirect = format!("http://127.0.0.1:{}", listener.local_addr()?.port());
    let verifier = random_token(48);
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let state = random_token(16);
    let url = format!(
        "{AUTH_URL}?client_id={}&redirect_uri={}&response_type=code&scope={}&code_challenge={challenge}\
         &code_challenge_method=S256&access_type=offline&prompt=consent&state={state}",
        urlencoding::encode(client_id),
        urlencoding::encode(&redirect),
        urlencoding::encode(SCOPE),
    );
    Ok(AuthRequest { listener, verifier, state, redirect, url })
}

/// The connected account.
pub struct Account {
    pub refresh_token: String,
    pub email: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
}

/// Waits for the browser redirect and exchanges the code for tokens.
pub fn finish_auth(req: AuthRequest, client_id: &str, client_secret: &str, cancel: Arc<AtomicBool>) -> Result<Account> {
    req.listener.set_nonblocking(true)?;
    let deadline = Instant::now() + AUTH_TIMEOUT;
    let params = loop {
        match req.listener.accept() {
            Ok((mut stream, _)) => {
                stream.set_nonblocking(false)?;
                stream.set_read_timeout(Some(Duration::from_secs(5)))?;
                let mut buf = Vec::new();
                let mut chunk = [0u8; 2048];
                while !buf.windows(4).any(|w| w == b"\r\n\r\n") && buf.len() < 16 * 1024 {
                    match stream.read(&mut chunk) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => buf.extend_from_slice(&chunk[..n]),
                    }
                }
                let request = String::from_utf8_lossy(&buf);
                let target = request.lines().next().and_then(|l| l.split_whitespace().nth(1)).unwrap_or("");
                let Some(query) = target.strip_prefix("/?") else {
                    // e.g. /favicon.ico
                    let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                    continue;
                };
                let params: HashMap<String, String> = query
                    .split('&')
                    .filter_map(|kv| kv.split_once('='))
                    .map(|(k, v)| (k.to_owned(), urlencoding::decode(v).map(|v| v.into_owned()).unwrap_or_default()))
                    .collect();
                let ok = params.contains_key("code");
                let message = if ok {
                    t!("QuickNotes is connected to Google Drive. You can close this tab.")
                } else {
                    t!("Connecting QuickNotes to Google Drive failed. You can close this tab.")
                };
                let page = format!(
                    "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>QuickNotes</title></head>\
                     <body style=\"font:16px system-ui,sans-serif;text-align:center;margin-top:20vh\">\
                     <h2>QuickNotes</h2><p>{message}</p></body></html>"
                );
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page}",
                    page.len()
                );
                break params;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if cancel.load(Ordering::Relaxed) || Instant::now() > deadline {
                    return Err(Error::Cancelled);
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(e.into()),
        }
    };
    if params.get("state") != Some(&req.state) {
        return Err(Error::Auth("state mismatch".into()));
    }
    if let Some(e) = params.get("error") {
        return Err(Error::Auth(e.clone()));
    }
    let code = params.get("code").ok_or_else(|| Error::Auth("no code".into()))?;
    let resp: TokenResponse = agent()
        .post(TOKEN_URL)
        .send_form(&[
            ("code", code),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("redirect_uri", &req.redirect),
            ("grant_type", "authorization_code"),
            ("code_verifier", &req.verifier),
        ])?
        .into_json()?;
    let refresh_token = resp.refresh_token.ok_or_else(|| Error::Auth("no refresh token".into()))?;
    let drive = Drive { agent: agent(), token: resp.access_token, ep: Endpoints::google() };
    let email = drive.email().unwrap_or_default();
    Ok(Account { refresh_token, email })
}

/// Revokes the authorisation (best effort).
pub fn revoke(refresh_token: &str) {
    let _ = agent().post(REVOKE_URL).send_form(&[("token", refresh_token)]);
}

// ===== Drive API ============================================================

#[derive(Deserialize, Debug, Clone)]
pub struct Remote {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "mimeType")]
    pub mime: String,
    /// Increases with every change of content or metadata.
    #[serde(default, deserialize_with = "version_from_str")]
    pub version: u64,
    #[serde(default, rename = "appProperties")]
    pub props: HashMap<String, String>,
}

fn version_from_str<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<u64, D::Error> {
    let s = String::deserialize(d)?;
    s.parse().map_err(serde::de::Error::custom)
}

impl Remote {
    /// Tab id of the file: the stored one, or one derived from the file id
    /// for files put into the folder by hand.
    pub fn note_id(&self) -> String {
        self.props.get("qn_id").cloned().unwrap_or_else(|| format!("gd-{}", self.id))
    }

    /// Files that can be notes: written by QuickNotes or plain text.
    fn usable(&self) -> bool {
        self.props.contains_key("qn_id") || self.mime.starts_with("text/") || self.mime == "application/json"
    }
}

/// API addresses (replaced by a mock server in the tests).
#[derive(Clone)]
struct Endpoints {
    token: String,
    files: String,
    upload: String,
    about: String,
}

impl Endpoints {
    fn google() -> Self {
        Endpoints { token: TOKEN_URL.into(), files: FILES_URL.into(), upload: UPLOAD_URL.into(), about: ABOUT_URL.into() }
    }
}

struct Drive {
    agent: ureq::Agent,
    token: String,
    ep: Endpoints,
}

impl Drive {
    fn connect(ep: Endpoints, client_id: &str, client_secret: &str, refresh_token: &str) -> Result<Self> {
        let agent = agent();
        let resp: TokenResponse = agent
            .post(&ep.token)
            .send_form(&[
                ("client_id", client_id),
                ("client_secret", client_secret),
                ("refresh_token", refresh_token),
                ("grant_type", "refresh_token"),
            ])?
            .into_json()?;
        Ok(Drive { agent, token: resp.access_token, ep })
    }

    fn get(&self, url: &str) -> ureq::Request {
        self.agent.get(url).set("Authorization", &format!("Bearer {}", self.token))
    }

    fn request(&self, method: &str, url: &str) -> ureq::Request {
        self.agent.request(method, url).set("Authorization", &format!("Bearer {}", self.token))
    }

    fn email(&self) -> Result<String> {
        let v: serde_json::Value = self.get(&self.ep.about).query("fields", "user(emailAddress)").call()?.into_json()?;
        Ok(v["user"]["emailAddress"].as_str().unwrap_or_default().to_owned())
    }

    /// The QuickNotes folder: the known one if it still exists, otherwise found or created.
    fn folder(&self, known: &str) -> Result<String> {
        if !known.is_empty() {
            match self.get(&format!("{}/{known}", self.ep.files)).query("fields", "id,trashed").call() {
                Ok(r) => {
                    let v: serde_json::Value = r.into_json()?;
                    if v["trashed"] != true {
                        return Ok(known.to_owned());
                    }
                }
                Err(ureq::Error::Status(404, _)) => {}
                Err(e) => return Err(e.into()),
            }
        }
        let q = format!("name = '{FOLDER_NAME}' and mimeType = '{FOLDER_MIME}' and trashed = false");
        let v: serde_json::Value =
            self.get(&self.ep.files).query("q", &q).query("fields", "files(id)").query("spaces", "drive").call()?.into_json()?;
        if let Some(id) = v["files"][0]["id"].as_str() {
            return Ok(id.to_owned());
        }
        let v: serde_json::Value = self
            .request("POST", &self.ep.files)
            .query("fields", "id")
            .send_json(serde_json::json!({ "name": FOLDER_NAME, "mimeType": FOLDER_MIME }))?
            .into_json()?;
        v["id"].as_str().map(str::to_owned).ok_or_else(|| Error::Net("no folder id".into()))
    }

    fn list(&self, folder: &str) -> Result<Vec<Remote>> {
        #[derive(Deserialize)]
        struct Page {
            #[serde(default)]
            files: Vec<Remote>,
            #[serde(rename = "nextPageToken")]
            next: Option<String>,
        }
        let q = format!("'{folder}' in parents and trashed = false");
        let mut out = Vec::new();
        let mut token: Option<String> = None;
        loop {
            let mut req = self
                .get(&self.ep.files)
                .query("q", &q)
                .query("pageSize", "1000")
                .query("fields", "nextPageToken,files(id,name,mimeType,version,appProperties)");
            if let Some(t) = &token {
                req = req.query("pageToken", t);
            }
            let page: Page = req.call()?.into_json()?;
            out.extend(page.files);
            match page.next {
                Some(t) => token = Some(t),
                None => break,
            }
        }
        Ok(out)
    }

    fn download(&self, id: &str) -> Result<String> {
        let mut bytes = Vec::new();
        self.get(&format!("{}/{id}", self.ep.files)).query("alt", "media").call()?.into_reader().read_to_end(&mut bytes)?;
        Ok(String::from_utf8(bytes).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned()))
    }

    fn upload(&self, method: &str, url: &str, meta: serde_json::Value, content: &str) -> Result<(String, u64)> {
        let boundary = format!("qn{}", random_token(18));
        let mut body = format!(
            "--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{meta}\r\n\
             --{boundary}\r\nContent-Type: text/plain; charset=UTF-8\r\n\r\n"
        )
        .into_bytes();
        body.extend_from_slice(content.as_bytes());
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        let v: serde_json::Value = self
            .request(method, url)
            .query("uploadType", "multipart")
            .query("fields", "id,version")
            .set("Content-Type", &format!("multipart/related; boundary={boundary}"))
            .send_bytes(&body)?
            .into_json()?;
        let id = v["id"].as_str().unwrap_or_default().to_owned();
        let version = v["version"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0);
        Ok((id, version))
    }

    fn create(&self, folder: &str, note: &Note, id: &str, name: &str, content: &str) -> Result<(String, u64)> {
        let mut meta = note.metadata(id, name);
        meta["parents"] = serde_json::json!([folder]);
        meta["mimeType"] = serde_json::json!(mime_for(name));
        self.upload("POST", &self.ep.upload, meta, content)
    }

    fn update(&self, drive_id: &str, note: &Note) -> Result<u64> {
        let meta = note.metadata(&note.id, &note.name);
        Ok(self.upload("PATCH", &format!("{}/{drive_id}", self.ep.upload), meta, &note.content)?.1)
    }

    fn trash(&self, drive_id: &str) -> Result<()> {
        match self.request("PATCH", &format!("{}/{drive_id}", self.ep.files)).send_json(serde_json::json!({ "trashed": true })) {
            Ok(_) | Err(ureq::Error::Status(404, _)) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

fn mime_for(name: &str) -> &'static str {
    if name.ends_with(".md") {
        "text/markdown"
    } else {
        "text/plain"
    }
}

// ===== sync =================================================================

/// What was synced last time, per tab id.
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct State {
    /// Account the state belongs to; a different account starts afresh.
    pub account: String,
    pub folder: String,
    pub files: BTreeMap<String, Synced>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Synced {
    pub drive_id: String,
    pub version: u64,
    /// Fingerprint of the synced title, language and content.
    pub hash: String,
}

fn state_path() -> std::path::PathBuf {
    crate::store::data_dir().join("drive-sync.json")
}

pub fn load_state() -> State {
    std::fs::read_to_string(state_path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

pub fn save_state(state: &State) {
    if let Ok(json) = serde_json::to_vec_pretty(state) {
        let path = state_path();
        let tmp = path.with_extension("tmp");
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(tmp, path);
        }
    }
}

/// A tab as it is synced.
#[derive(Clone, Debug)]
pub struct Note {
    pub id: String,
    /// File name on Drive.
    pub name: String,
    /// User-defined tab title.
    pub title: Option<String>,
    pub language: Option<String>,
    pub content: String,
}

impl Note {
    pub fn hash(&self) -> String {
        fingerprint(self.title.as_deref(), self.language.as_deref(), &self.content)
    }

    fn metadata(&self, id: &str, name: &str) -> serde_json::Value {
        // appProperties allow 124 bytes for key and value together.
        let title = self.title.as_deref().map(|t| cut(t, 100));
        serde_json::json!({
            "name": name,
            "appProperties": { "qn_id": id, "qn_lang": self.language, "qn_title": title },
        })
    }
}

fn cut(s: &str, max: usize) -> &str {
    let mut end = s.len().min(max);
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Stable fingerprint (FNV-1a, 64 bit) of what is synced for a tab.
pub fn fingerprint(title: Option<&str>, language: Option<&str>, content: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for part in [title.unwrap_or(""), "\u{0}", language.unwrap_or(""), "\u{0}", content] {
        for b in part.bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    format!("{h:016x}")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// At start: fetch changes from Drive.
    Pull,
    /// When closing: send local changes to Drive.
    Push,
}

/// How a downloaded file ends up in the tabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Apply {
    /// Replace the content of the existing tab.
    Replace,
    /// A new tab with the file's id.
    New,
    /// Both sides changed: a new tab with the Drive version (fresh id); the
    /// local version wins in the existing tab and is uploaded later.
    Copy,
    /// Same id on both sides without a sync record: link them if equal,
    /// otherwise like `Copy`.
    Link,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    /// Upload the local tab (create when `drive_id` is `None`).
    Upload { id: String, drive_id: Option<String> },
    /// Both changed while closing: keep the Drive version as a conflict copy
    /// file, then upload the local tab over it.
    ConflictUpload { id: String, drive_id: String },
    Download { id: String, drive_id: String, apply: Apply },
    /// The tab was closed locally: move the file to the Drive trash.
    Trash { id: String, drive_id: String },
    /// The file was deleted on Drive and the tab is unchanged: close it.
    Close { id: String },
    /// Drop the sync record (the tab is uploaded as a new file later).
    Forget { id: String },
}

/// Decides what to do for every tab and file. `local` maps tab ids to their
/// current fingerprint; tabs in `skip` exist but are left alone (still
/// loading, too large), so they don't count as deleted.
pub fn plan(
    local: &BTreeMap<String, String>,
    skip: &HashSet<String>,
    state: &State,
    remote: &[Remote],
    mode: Mode,
) -> Vec<Step> {
    // Per note id the newest file (duplicates can appear when two machines
    // create the same note at the same time).
    let mut files: BTreeMap<String, &Remote> = BTreeMap::new();
    for r in remote.iter().filter(|r| r.usable()) {
        let id = r.note_id();
        if files.get(&id).is_none_or(|old| old.version < r.version) {
            files.insert(id, r);
        }
    }
    let ids: HashSet<&String> = local.keys().chain(state.files.keys()).chain(files.keys()).collect();
    let mut ids: Vec<&String> = ids.into_iter().filter(|id| !skip.contains(*id)).collect();
    ids.sort();

    let pull = mode == Mode::Pull;
    let push = mode == Mode::Push;
    let mut steps = Vec::new();
    for id in ids {
        let id = id.clone();
        let (l, s, r) = (local.get(&id), state.files.get(&id), files.get(&id));
        let step = match (l, s, r) {
            (Some(_), None, None) if push => Some(Step::Upload { id, drive_id: None }),
            (Some(_), None, Some(r)) if pull => Some(Step::Download { id, drive_id: r.id.clone(), apply: Apply::Link }),
            (Some(_), None, Some(r)) if push => Some(Step::ConflictUpload { id, drive_id: r.id.clone() }),
            (Some(hash), Some(s), Some(r)) => {
                let local_changed = *hash != s.hash;
                let remote_changed = r.version != s.version || r.id != s.drive_id;
                let drive_id = r.id.clone();
                match (local_changed, remote_changed) {
                    (false, false) => None,
                    (true, false) if push => Some(Step::Upload { id, drive_id: Some(drive_id) }),
                    (false, true) if pull => Some(Step::Download { id, drive_id, apply: Apply::Replace }),
                    (true, true) if pull => Some(Step::Download { id, drive_id, apply: Apply::Copy }),
                    (true, true) if push => Some(Step::ConflictUpload { id, drive_id }),
                    _ => None,
                }
            }
            (Some(hash), Some(s), None) => {
                let local_changed = *hash != s.hash;
                match (local_changed, pull) {
                    (false, true) => Some(Step::Close { id }),
                    (true, _) => Some(Step::Forget { id }),
                    (false, false) => None,
                }
            }
            (None, Some(s), Some(r)) => {
                let remote_changed = r.version != s.version;
                match (remote_changed, pull) {
                    (false, false) => Some(Step::Trash { id, drive_id: r.id.clone() }),
                    (true, true) => Some(Step::Download { id, drive_id: r.id.clone(), apply: Apply::New }),
                    (true, false) => Some(Step::Forget { id }),
                    (false, true) => None,
                }
            }
            (None, None, Some(r)) if pull => Some(Step::Download { id, drive_id: r.id.clone(), apply: Apply::New }),
            (None, Some(_), None) => Some(Step::Forget { id }),
            _ => None,
        };
        steps.extend(step);
    }
    steps
}

/// A downloaded file to show in a tab.
#[derive(Debug)]
pub struct Pulled {
    pub id: String,
    pub title: Option<String>,
    pub language: Option<String>,
    pub content: String,
    pub apply: Apply,
    /// File name on Drive (title for conflict copies).
    pub name: String,
}

#[derive(Debug, Default)]
pub struct Outcome {
    pub state: State,
    pub pulled: Vec<Pulled>,
    pub closed: Vec<String>,
    pub uploaded: usize,
    pub trashed: usize,
    pub conflicts: usize,
}

/// Credentials for a sync run; the secrets come from the keyring.
pub struct Credentials {
    pub client_id: String,
    pub account: String,
}

/// Runs one sync (blocking).
pub fn sync(creds: &Credentials, notes: Vec<Note>, skip: HashSet<String>, mut state: State, mode: Mode) -> Result<Outcome> {
    let secret_value = secret::load(secret::CLIENT_SECRET).ok_or_else(|| Error::Auth(t!("Client secret missing").into()))?;
    let refresh = secret::load(secret::REFRESH_TOKEN).ok_or_else(|| Error::Auth(t!("Not connected").into()))?;
    let drive = Drive::connect(Endpoints::google(), &creds.client_id, &secret_value, &refresh)?;
    run(&drive, &creds.account, notes, skip, state, mode)
}

fn run(drive: &Drive, account: &str, notes: Vec<Note>, skip: HashSet<String>, mut state: State, mode: Mode) -> Result<Outcome> {
    if state.account != account {
        state = State { account: account.to_owned(), ..Default::default() };
    }
    state.folder = drive.folder(&state.folder)?;
    let remote = drive.list(&state.folder)?;
    let by_drive_id: HashMap<&str, &Remote> = remote.iter().map(|r| (r.id.as_str(), r)).collect();
    let notes: BTreeMap<String, Note> = notes.into_iter().map(|n| (n.id.clone(), n)).collect();
    let local: BTreeMap<String, String> = notes.iter().map(|(id, n)| (id.clone(), n.hash())).collect();

    let mut out = Outcome::default();
    for step in plan(&local, &skip, &state, &remote, mode) {
        match step {
            Step::Upload { id, drive_id } => {
                let note = &notes[&id];
                let (drive_id, version) = match drive_id {
                    Some(d) => {
                        let v = drive.update(&d, note)?;
                        (d, v)
                    }
                    None => drive.create(&state.folder, note, &note.id, &note.name, &note.content)?,
                };
                state.files.insert(id, Synced { drive_id, version, hash: note.hash() });
                out.uploaded += 1;
            }
            Step::ConflictUpload { id, drive_id } => {
                let note = &notes[&id];
                let theirs = drive.download(&drive_id)?;
                let r = by_drive_id.get(drive_id.as_str());
                let theirs_note = Note {
                    id: crate::store::new_id(),
                    name: conflict_name(&note.name),
                    title: r.and_then(|r| r.props.get("qn_title").cloned()),
                    language: r.and_then(|r| r.props.get("qn_lang").cloned()),
                    content: theirs,
                };
                if theirs_note.hash() != note.hash() {
                    drive.create(&state.folder, &theirs_note, &theirs_note.id, &theirs_note.name, &theirs_note.content)?;
                    out.conflicts += 1;
                }
                let version = drive.update(&drive_id, note)?;
                state.files.insert(id, Synced { drive_id, version, hash: note.hash() });
                out.uploaded += 1;
            }
            Step::Download { id, drive_id, apply } => {
                let Some(r) = by_drive_id.get(drive_id.as_str()) else { continue };
                let content = drive.download(&drive_id)?;
                let title = r.props.get("qn_title").cloned();
                let language = r.props.get("qn_lang").cloned();
                let hash = fingerprint(title.as_deref(), language.as_deref(), &content);
                let synced = Synced { drive_id: drive_id.clone(), version: r.version, hash: hash.clone() };
                let apply = match apply {
                    Apply::Link if local.get(&id) == Some(&hash) => {
                        state.files.insert(id, synced);
                        continue;
                    }
                    Apply::Link => Apply::Copy,
                    a => a,
                };
                match apply {
                    Apply::Copy => {
                        // The local version stays and is uploaded next time.
                        let local_hash = state.files.get(&id).map(|s| s.hash.clone()).unwrap_or_default();
                        state.files.insert(id, Synced { hash: local_hash, ..synced });
                        out.conflicts += 1;
                        out.pulled.push(Pulled {
                            id: crate::store::new_id(),
                            title,
                            language,
                            content,
                            apply,
                            name: r.name.clone(),
                        });
                    }
                    _ => {
                        state.files.insert(id.clone(), synced);
                        out.pulled.push(Pulled { id, title, language, content, apply, name: r.name.clone() });
                    }
                }
            }
            Step::Trash { id, drive_id } => {
                drive.trash(&drive_id)?;
                state.files.remove(&id);
                out.trashed += 1;
            }
            Step::Close { id } => {
                state.files.remove(&id);
                out.closed.push(id);
            }
            Step::Forget { id } => {
                state.files.remove(&id);
            }
        }
    }
    out.state = state;
    Ok(out)
}

fn conflict_name(name: &str) -> String {
    let stamp = glib_now();
    match name.split_once('.') {
        Some((stem, ext)) => format!("{stem} ({} {stamp}).{ext}", t!("conflict")),
        None => format!("{name} ({} {stamp})", t!("conflict")),
    }
}

fn glib_now() -> String {
    gtk::glib::DateTime::now_local()
        .and_then(|d| d.format("%Y-%m-%d %H:%M"))
        .map(|s| s.to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remote(id: &str, note: &str, version: u64) -> Remote {
        Remote {
            id: id.into(),
            name: format!("{note}.txt"),
            mime: "text/plain".into(),
            version,
            props: HashMap::from([("qn_id".to_owned(), note.to_owned())]),
        }
    }

    fn synced(drive: &str, version: u64, hash: &str) -> Synced {
        Synced { drive_id: drive.into(), version, hash: hash.into() }
    }

    fn none() -> HashSet<String> {
        HashSet::new()
    }

    fn local(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
    }

    fn state(entries: &[(&str, Synced)]) -> State {
        State { files: entries.iter().map(|(k, v)| (k.to_string(), v.clone())).collect(), ..Default::default() }
    }

    #[test]
    fn new_notes_go_up_and_new_files_come_down() {
        let l = local(&[("a", "h1")]);
        let r = [remote("D2", "b", 3)];
        assert_eq!(plan(&l, &none(), &State::default(), &r, Mode::Push), [Step::Upload { id: "a".into(), drive_id: None }]);
        assert_eq!(
            plan(&l, &none(), &State::default(), &r, Mode::Pull),
            [Step::Download { id: "b".into(), drive_id: "D2".into(), apply: Apply::New }]
        );
    }

    #[test]
    fn changes_flow_in_the_right_direction() {
        let s = state(&[("a", synced("D1", 5, "h1"))]);
        // Local change.
        let l = local(&[("a", "h2")]);
        let r = [remote("D1", "a", 5)];
        assert_eq!(plan(&l, &none(), &s, &r, Mode::Push), [Step::Upload { id: "a".into(), drive_id: Some("D1".into()) }]);
        assert_eq!(plan(&l, &none(), &s, &r, Mode::Pull), []);
        // Remote change.
        let l = local(&[("a", "h1")]);
        let r = [remote("D1", "a", 6)];
        assert_eq!(plan(&l, &none(), &s, &r, Mode::Push), []);
        assert_eq!(
            plan(&l, &none(), &s, &r, Mode::Pull),
            [Step::Download { id: "a".into(), drive_id: "D1".into(), apply: Apply::Replace }]
        );
        // Unchanged.
        let r = [remote("D1", "a", 5)];
        assert_eq!(plan(&l, &none(), &s, &r, Mode::Push), []);
        assert_eq!(plan(&l, &none(), &s, &r, Mode::Pull), []);
    }

    #[test]
    fn conflicts_keep_both_versions() {
        let s = state(&[("a", synced("D1", 5, "h1"))]);
        let l = local(&[("a", "h2")]);
        let r = [remote("D1", "a", 6)];
        assert_eq!(
            plan(&l, &none(), &s, &r, Mode::Pull),
            [Step::Download { id: "a".into(), drive_id: "D1".into(), apply: Apply::Copy }]
        );
        assert_eq!(plan(&l, &none(), &s, &r, Mode::Push), [Step::ConflictUpload { id: "a".into(), drive_id: "D1".into() }]);
    }

    #[test]
    fn deletions() {
        let s = state(&[("a", synced("D1", 5, "h1"))]);
        // Closed locally: trashed on Drive when closing, unless changed there.
        let r = [remote("D1", "a", 5)];
        assert_eq!(
            plan(&local(&[]), &none(), &s, &r, Mode::Push),
            [Step::Trash { id: "a".into(), drive_id: "D1".into() }]
        );
        let r = [remote("D1", "a", 7)];
        assert_eq!(
            plan(&local(&[]), &none(), &s, &r, Mode::Pull),
            [Step::Download { id: "a".into(), drive_id: "D1".into(), apply: Apply::New }]
        );
        // Deleted on Drive: unchanged tabs close, changed ones are uploaded again.
        assert_eq!(plan(&local(&[("a", "h1")]), &none(), &s, &[], Mode::Pull), [Step::Close { id: "a".into() }]);
        assert_eq!(plan(&local(&[("a", "h1")]), &none(), &s, &[], Mode::Push), []);
        assert_eq!(plan(&local(&[("a", "h9")]), &none(), &s, &[], Mode::Push), [Step::Forget { id: "a".into() }]);
        assert_eq!(plan(&local(&[]), &none(), &s, &[], Mode::Push), [Step::Forget { id: "a".into() }]);
    }

    #[test]
    fn skipped_tabs_are_not_deleted() {
        let s = state(&[("a", synced("D1", 5, "h1"))]);
        let skip = HashSet::from(["a".to_owned()]);
        assert_eq!(plan(&local(&[]), &skip, &s, &[remote("D1", "a", 5)], Mode::Push), []);
    }

    #[test]
    fn unknown_pairs_are_linked_or_copied() {
        let l = local(&[("a", "h1")]);
        let r = [remote("D1", "a", 2)];
        assert_eq!(
            plan(&l, &none(), &State::default(), &r, Mode::Pull),
            [Step::Download { id: "a".into(), drive_id: "D1".into(), apply: Apply::Link }]
        );
    }

    #[test]
    fn hand_made_files_and_duplicates() {
        let mut plain = remote("D7", "x", 1);
        plain.props.clear();
        let mut binary = plain.clone();
        binary.id = "D8".into();
        binary.mime = "image/png".into();
        let steps = plan(&local(&[]), &none(), &State::default(), &[plain, binary], Mode::Pull);
        assert_eq!(steps, [Step::Download { id: "gd-D7".into(), drive_id: "D7".into(), apply: Apply::New }]);
        // Two files for one note: the newer one wins.
        let steps = plan(&local(&[]), &none(), &State::default(), &[remote("D1", "a", 3), remote("D2", "a", 9)], Mode::Pull);
        assert_eq!(steps, [Step::Download { id: "a".into(), drive_id: "D2".into(), apply: Apply::New }]);
    }

    #[test]
    fn fingerprints_cover_title_language_and_content() {
        let base = fingerprint(None, None, "x");
        assert_ne!(base, fingerprint(Some("t"), None, "x"));
        assert_ne!(base, fingerprint(None, Some("markdown"), "x"));
        assert_ne!(base, fingerprint(None, None, "y"));
        assert_eq!(base, fingerprint(None, None, "x"));
        assert_eq!(base.len(), 16);
    }

    /// Simulates the browser redirect to the loopback listener.
    fn redirect(query: &str, state_ok: bool) -> Result<Account> {
        let req = begin_auth("client").unwrap();
        assert!(req.url.contains("code_challenge_method=S256") && req.url.contains("access_type=offline"));
        let port = req.listener.local_addr().unwrap().port();
        let state = if state_ok { req.state.clone() } else { "wrong".into() };
        let query = format!("{query}&state={state}");
        let client = std::thread::spawn(move || {
            let send = |path: &str| {
                let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
                write!(s, "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
                let mut answer = String::new();
                let _ = s.read_to_string(&mut answer);
                answer
            };
            assert!(send("/favicon.ico").starts_with("HTTP/1.1 404"));
            send(&format!("/?{query}"))
        });
        let result = finish_auth(req, "client", "secret", Arc::new(AtomicBool::new(false)));
        assert!(client.join().unwrap().starts_with("HTTP/1.1 200"));
        result
    }

    #[test]
    fn loopback_redirect_is_checked() {
        assert!(matches!(redirect("code=abc", false), Err(Error::Auth(e)) if e == "state mismatch"));
        assert!(matches!(redirect("error=access_denied", true), Err(Error::Auth(e)) if e == "access_denied"));
    }

    #[test]
    fn cancelled_sign_in() {
        let req = begin_auth("client").unwrap();
        let cancel = Arc::new(AtomicBool::new(true));
        assert!(matches!(finish_auth(req, "c", "s", cancel), Err(Error::Cancelled)));
    }

    #[test]
    fn remote_versions_parse_from_strings() {
        let r: Remote = serde_json::from_str(r#"{"id":"x","version":"42","appProperties":{"qn_id":"a"}}"#).unwrap();
        assert_eq!((r.version, r.note_id()), (42, "a".to_owned()));
    }

    // ----- end-to-end against a mock of the Drive API ---------------------

    #[derive(Clone, Default)]
    struct MockFile {
        name: String,
        mime: String,
        version: u64,
        props: serde_json::Map<String, serde_json::Value>,
        content: String,
        trashed: bool,
        parents: Vec<String>,
    }

    type Files = std::sync::Arc<std::sync::Mutex<BTreeMap<String, MockFile>>>;

    /// A tiny HTTP server that implements the Drive calls used here.
    fn mock_drive() -> (Endpoints, Files) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        let files: Files = Default::default();
        let store = files.clone();
        std::thread::spawn(move || {
            let mut next = 0;
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let (method, target, body) = read_request(&mut stream);
                let (path, query) = target.split_once('?').unwrap_or((&target, ""));
                let q: HashMap<String, String> = query
                    .split('&')
                    .filter_map(|kv| kv.split_once('='))
                    .map(|(k, v)| (k.to_owned(), urlencoding::decode(&v.replace('+', " ")).unwrap().into_owned()))
                    .collect();
                let mut files = store.lock().unwrap();
                let json = |f: &MockFile, id: &str| {
                    serde_json::json!({ "id": id, "name": f.name, "mimeType": f.mime,
                        "version": f.version.to_string(), "appProperties": f.props, "trashed": f.trashed })
                };
                let (status, reply): (u16, String) = match (method.as_str(), path) {
                    ("POST", "/token") => (200, r#"{"access_token":"tok"}"#.into()),
                    ("GET", "/files") => {
                        let query = &q["q"];
                        let list: Vec<serde_json::Value> = files
                            .iter()
                            .filter(|(_, f)| !f.trashed)
                            .filter(|(_, f)| {
                                if query.contains("in parents") {
                                    f.parents.iter().any(|p| query.contains(p.as_str()))
                                } else {
                                    f.mime == FOLDER_MIME && f.name == FOLDER_NAME
                                }
                            })
                            .map(|(id, f)| json(f, id))
                            .collect();
                        (200, serde_json::json!({ "files": list }).to_string())
                    }
                    ("POST", "/files") => {
                        let meta: serde_json::Value = serde_json::from_slice(&body).unwrap();
                        next += 1;
                        let id = format!("F{next}");
                        let f = MockFile {
                            name: meta["name"].as_str().unwrap().into(),
                            mime: meta["mimeType"].as_str().unwrap().into(),
                            version: 1,
                            ..Default::default()
                        };
                        files.insert(id.clone(), f);
                        (200, serde_json::json!({ "id": id }).to_string())
                    }
                    ("GET", p) if p.starts_with("/files/") => {
                        let id = &p[7..];
                        match files.get(id) {
                            Some(f) if q.get("alt").map(String::as_str) == Some("media") => (200, f.content.clone()),
                            Some(f) => (200, json(f, id).to_string()),
                            None => (404, "{}".into()),
                        }
                    }
                    ("PATCH", p) if p.starts_with("/files/") => {
                        let meta: serde_json::Value = serde_json::from_slice(&body).unwrap();
                        let f = files.get_mut(&p[7..]).unwrap();
                        f.trashed = meta["trashed"] == true;
                        f.version += 1;
                        (200, "{}".into())
                    }
                    (m @ ("POST" | "PATCH"), p) if p.starts_with("/upload") => {
                        let (meta, content) = split_multipart(&body);
                        let id = if m == "POST" {
                            next += 1;
                            let id = format!("D{next}");
                            let parents = meta["parents"].as_array().unwrap().iter().map(|p| p.as_str().unwrap().to_owned()).collect();
                            let mime = meta["mimeType"].as_str().unwrap().to_owned();
                            files.insert(id.clone(), MockFile { parents, mime, ..Default::default() });
                            id
                        } else {
                            p.trim_start_matches("/upload/").to_owned()
                        };
                        let f = files.get_mut(&id).unwrap();
                        f.name = meta["name"].as_str().unwrap().into();
                        for (k, v) in meta["appProperties"].as_object().unwrap() {
                            if v.is_null() {
                                f.props.remove(k);
                            } else {
                                f.props.insert(k.clone(), v.clone());
                            }
                        }
                        f.content = content;
                        f.version += 1;
                        (200, serde_json::json!({ "id": id, "version": f.version.to_string() }).to_string())
                    }
                    _ => (404, "{}".into()),
                };
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                    reply.len()
                );
            }
        });
        let ep = Endpoints {
            token: format!("{base}/token"),
            files: format!("{base}/files"),
            upload: format!("{base}/upload"),
            about: format!("{base}/about"),
        };
        (ep, files)
    }

    fn read_request(stream: &mut std::net::TcpStream) -> (String, String, Vec<u8>) {
        let mut buf = Vec::new();
        let mut byte = [0u8; 1];
        while !buf.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).unwrap();
            buf.push(byte[0]);
        }
        let head = String::from_utf8_lossy(&buf).to_string();
        let mut first = head.lines().next().unwrap().split_whitespace();
        let (method, target) = (first.next().unwrap().to_owned(), first.next().unwrap().to_owned());
        let len = head
            .lines()
            .find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse().unwrap()))
            .unwrap_or(0);
        let mut body = vec![0u8; len];
        stream.read_exact(&mut body).unwrap();
        (method, target, body)
    }

    fn split_multipart(body: &[u8]) -> (serde_json::Value, String) {
        let text = String::from_utf8(body.to_vec()).unwrap();
        let boundary = text.lines().next().unwrap().to_owned();
        let parts: Vec<&str> = text.split(boundary.as_str()).collect();
        let part = |p: &str| p.split_once("\r\n\r\n").unwrap().1.strip_suffix("\r\n").unwrap().to_owned();
        (serde_json::from_str(&part(parts[1])).unwrap(), part(parts[2]))
    }

    fn note(id: &str, content: &str) -> Note {
        Note { id: id.into(), name: format!("{id}.txt"), title: None, language: None, content: content.into() }
    }

    /// One machine: its tabs and its sync record.
    struct Machine {
        notes: Vec<Note>,
        state: State,
    }

    impl Machine {
        fn sync(&mut self, drive: &Drive, mode: Mode) -> Outcome {
            let out = run(drive, "me", self.notes.clone(), HashSet::new(), self.state.clone(), mode).unwrap();
            // Apply like the window does.
            for p in &out.pulled {
                match p.apply {
                    Apply::Replace => {
                        let n = self.notes.iter_mut().find(|n| n.id == p.id).unwrap();
                        n.content = p.content.clone();
                        n.title = p.title.clone();
                        n.language = p.language.clone();
                    }
                    _ => self.notes.push(Note {
                        id: p.id.clone(),
                        name: p.name.clone(),
                        title: p.title.clone(),
                        language: p.language.clone(),
                        content: p.content.clone(),
                    }),
                }
            }
            self.notes.retain(|n| !out.closed.contains(&n.id));
            self.state = out.state.clone();
            out
        }

        fn content(&self, id: &str) -> Option<&str> {
            self.notes.iter().find(|n| n.id == id).map(|n| n.content.as_str())
        }

        fn edit(&mut self, id: &str, text: &str) {
            self.notes.iter_mut().find(|n| n.id == id).unwrap().content = text.into();
        }
    }

    #[test]
    fn two_machines_stay_in_sync() {
        let (ep, files) = mock_drive();
        let drive = Drive::connect(ep, "c", "s", "r").unwrap();
        let mut a = Machine {
            notes: vec![note("n1", "first"), Note { language: Some("markdown".into()), title: Some("Ideas".into()), ..note("n2", "# x") }],
            state: State::default(),
        };
        let mut b = Machine { notes: vec![], state: State::default() };

        // A creates the folder and uploads both notes; B receives them.
        assert_eq!(a.sync(&drive, Mode::Push).uploaded, 2);
        assert_eq!(a.sync(&drive, Mode::Push).uploaded, 0, "nothing changed");
        assert_eq!(b.sync(&drive, Mode::Pull).pulled.len(), 2);
        let n2 = b.notes.iter().find(|n| n.id == "n2").unwrap();
        assert_eq!((n2.title.as_deref(), n2.language.as_deref(), n2.content.as_str()), (Some("Ideas"), Some("markdown"), "# x"));
        assert_eq!(b.sync(&drive, Mode::Pull).pulled.len(), 0, "nothing new");

        // B edits, A receives the edit.
        b.edit("n1", "first, edited on B");
        assert_eq!(b.sync(&drive, Mode::Push).uploaded, 1);
        a.sync(&drive, Mode::Pull);
        assert_eq!(a.content("n1"), Some("first, edited on B"));

        // Both edit n1: A keeps its version and gets B's as a copy.
        a.edit("n1", "A's version");
        b.edit("n1", "B's version");
        b.sync(&drive, Mode::Push);
        let out = a.sync(&drive, Mode::Pull);
        assert_eq!(out.conflicts, 1);
        assert_eq!(a.content("n1"), Some("A's version"));
        assert!(a.notes.iter().any(|n| n.content == "B's version" && n.id != "n1"));
        // A sends its version and the copy; B gets both.
        assert_eq!(a.sync(&drive, Mode::Push).uploaded, 2);
        b.sync(&drive, Mode::Pull);
        assert_eq!(b.content("n1"), Some("A's version"));
        assert_eq!(b.notes.len(), 3);

        // Closing a tab on A trashes the file; B closes it, too.
        a.notes.retain(|n| n.id != "n2");
        assert_eq!(a.sync(&drive, Mode::Push).trashed, 1);
        assert!(files.lock().unwrap().values().any(|f| f.trashed && f.name == "n2.txt"));
        let out = b.sync(&drive, Mode::Pull);
        assert_eq!(out.closed, ["n2"]);
        assert_eq!(b.notes.len(), 2);

        // Conflict while closing: Drive keeps the other version as a conflict file.
        a.edit("n1", "A again");
        b.edit("n1", "B again");
        a.sync(&drive, Mode::Push);
        let out = b.sync(&drive, Mode::Push);
        assert_eq!(out.conflicts, 1);
        let files = files.lock().unwrap();
        assert!(files.values().any(|f| !f.trashed && f.content == "A again" && f.name.contains("conflict")));
        assert!(files.values().any(|f| !f.trashed && f.content == "B again" && f.props["qn_id"] == "n1"));
    }

}
