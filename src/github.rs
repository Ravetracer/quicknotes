//! Sync of the notes with a GitHub repository.
//!
//! - Sign-in with the OAuth device flow of QuickNotes' own OAuth app (the user
//!   enters a short code on github.com, scope `repo`) or with a personal
//!   access token (fine-grained: *Contents* read and write). The token is kept
//!   in the Secret Service (GNOME keyring), never in the settings.
//! - Everything is encrypted, private repository or not. The data key is
//!   random; `quicknotes/vault.json` holds it wrapped with the sync password
//!   (Argon2id, see [`crate::crypto::Lock`]). Each machine asks for the
//!   password once and keeps the key in the keyring.
//! - Layout in the repository:
//!   - `quicknotes/vault.json`      the wrapped key (readable)
//!   - `quicknotes/index.enc`       folders and the metadata of all notes
//!   - `quicknotes/notes/<id>.enc`  the content of each note
//! - Every sync is one commit made with the Git data API. When another
//!   machine pushed in between, the branch update is refused and the sync
//!   starts over with the new state.
//! - Changes are merged three-way against what was synced last time
//!   ([`State`]): metadata field by field, contents as a whole. When both
//!   sides changed a note, both versions are kept.
//!
//! Network calls block; they run in a worker thread (see `window.rs`).

use crate::crypto::{Key, Lock};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Client id of QuickNotes' OAuth app (device flow; there is no secret).
pub const CLIENT_ID: &str = "Ov23liuwqJ7XOCwSj2HF";

const API_URL: &str = "https://api.github.com";
const LOGIN_URL: &str = "https://github.com";
/// Where fine-grained tokens are created.
pub const TOKEN_PAGE: &str = "https://github.com/settings/personal-access-tokens/new";
const DIR: &str = "quicknotes/";
const VAULT: &str = "quicknotes/vault.json";
const INDEX: &str = "quicknotes/index.enc";
const INDEX_AAD: &[u8] = b"quicknotes-sync-index";
const FORMAT: u32 = 1;
/// Attempts when other machines push at the same time.
const TRIES: usize = 4;

#[derive(Debug)]
pub enum Error {
    /// The token is missing, expired or was revoked: sign in again.
    Auth(String),
    /// The repository is missing, not writable or not usable for QuickNotes.
    Repo(String),
    /// The stored key does not open the repository's data (the sync
    /// password was set up anew elsewhere).
    Key,
    Net(String),
    Cancelled,
    /// The branch moved while syncing (internal: the sync starts over).
    Moved,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Auth(e) => write!(f, "{}: {e}", t!("GitHub sign-in failed")),
            Error::Repo(e) | Error::Net(e) => write!(f, "{e}"),
            Error::Key => write!(f, "{}", t!("The sync key does not fit the repository – connect it again in the preferences")),
            Error::Cancelled => write!(f, "{}", t!("Cancelled")),
            Error::Moved => write!(f, "{}", t!("The repository keeps changing – please try again later")),
        }
    }
}

/// The `message` of a GitHub error response.
fn message(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v["message"].as_str().map(str::to_owned))
        .unwrap_or_else(|| body.chars().take(300).collect())
}

impl From<ureq::Error> for Error {
    fn from(e: ureq::Error) -> Self {
        match e {
            ureq::Error::Status(code, resp) => {
                let text = message(&resp.into_string().unwrap_or_default());
                match code {
                    401 => Error::Auth(text),
                    404 => Error::Repo(t!("Repository not found or no access").into()),
                    _ => Error::Net(format!("HTTP {code}: {text}")),
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

/// Values in the Secret Service: the token and one data key per repository.
pub mod secret {
    use gtk::gio;
    use std::collections::HashMap;

    pub const TOKEN: &str = "token";

    /// Name of the data key of `repo`.
    pub fn key_name(repo: &str) -> String {
        format!("key:{}", repo.to_lowercase())
    }

    fn schema() -> libsecret::Schema {
        let attributes = HashMap::from([("kind", libsecret::SchemaAttributeType::String)]);
        libsecret::Schema::new("io.github.quicknotes.GitHub", libsecret::SchemaFlags::NONE, attributes)
    }

    pub fn store(kind: &str, value: &str) -> Result<(), String> {
        let label = format!("QuickNotes – GitHub ({kind})");
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

// ===== API ==================================================================

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout(Duration::from_secs(120))
        .user_agent("QuickNotes")
        .build()
}

fn b64() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

/// Decodes GitHub's Base64 (with line breaks).
fn decode(content: &str) -> Result<String> {
    let clean: String = content.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = b64().decode(clean).map_err(|e| Error::Net(e.to_string()))?;
    String::from_utf8(bytes).map_err(|e| Error::Net(e.to_string()))
}

/// Addresses of the API and the sign-in (replaced by a mock server in the tests).
#[derive(Clone)]
struct Endpoints {
    api: String,
    login: String,
}

impl Endpoints {
    fn github() -> Self {
        Endpoints { api: API_URL.into(), login: LOGIN_URL.into() }
    }
}

/// `owner/name` from what the user typed: also a repository URL or an SSH address.
pub fn parse_repo(input: &str) -> Option<String> {
    let s = input.trim().trim_end_matches('/');
    let s = s.strip_suffix(".git").unwrap_or(s);
    let s = ["https://github.com/", "http://github.com/", "github.com/", "git@github.com:"]
        .iter()
        .find_map(|p| s.strip_prefix(p))
        .unwrap_or(s);
    let (owner, name) = s.split_once('/')?;
    let ok = |part: &str| !part.is_empty() && part.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c));
    (ok(owner) && ok(name) && name != "." && name != "..").then(|| format!("{owner}/{name}"))
}

/// A started device sign-in: show `user_code`, open `verification_uri`,
/// then call [`device_finish`].
#[derive(Deserialize, Clone, Debug)]
pub struct DeviceCode {
    device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    #[serde(default = "five")]
    interval: u64,
    #[serde(default = "fifteen_minutes")]
    expires_in: u64,
}

fn five() -> u64 {
    5
}

fn fifteen_minutes() -> u64 {
    900
}

pub fn device_start() -> Result<DeviceCode> {
    device_start_at(&Endpoints::github(), CLIENT_ID)
}

fn device_start_at(ep: &Endpoints, client_id: &str) -> Result<DeviceCode> {
    let v: serde_json::Value = agent()
        .post(&format!("{}/login/device/code", ep.login))
        .set("Accept", "application/json")
        .send_form(&[("client_id", client_id), ("scope", "repo")])?
        .into_json()?;
    if let Some(e) = v["error"].as_str() {
        return Err(Error::Auth(v["error_description"].as_str().unwrap_or(e).to_owned()));
    }
    serde_json::from_value(v).map_err(|e| Error::Net(e.to_string()))
}

/// Waits until the code was entered on github.com; the access token.
pub fn device_finish(code: &DeviceCode, cancel: Arc<AtomicBool>) -> Result<String> {
    device_finish_at(&Endpoints::github(), CLIENT_ID, code, cancel)
}

fn device_finish_at(ep: &Endpoints, client_id: &str, code: &DeviceCode, cancel: Arc<AtomicBool>) -> Result<String> {
    let deadline = Instant::now() + Duration::from_secs(code.expires_in);
    let mut interval = code.interval;
    let agent = agent();
    loop {
        let wait_until = Instant::now() + Duration::from_secs(interval);
        while Instant::now() < wait_until {
            if cancel.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        if Instant::now() > deadline {
            return Err(Error::Auth(t!("The code has expired").into()));
        }
        let v: serde_json::Value = agent
            .post(&format!("{}/login/oauth/access_token", ep.login))
            .set("Accept", "application/json")
            .send_form(&[
                ("client_id", client_id),
                ("device_code", &code.device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])?
            .into_json()?;
        if let Some(token) = v["access_token"].as_str() {
            return Ok(token.to_owned());
        }
        match v["error"].as_str().unwrap_or("") {
            "authorization_pending" => {}
            "slow_down" => interval = v["interval"].as_u64().unwrap_or(interval + 5),
            "expired_token" => return Err(Error::Auth(t!("The code has expired").into())),
            "access_denied" => return Err(Error::Auth(t!("Access was denied").into())),
            e => return Err(Error::Auth(v["error_description"].as_str().unwrap_or(e).to_owned())),
        }
    }
}

/// The login name of the token's account.
pub fn user(token: &str) -> Result<String> {
    Client::new(Endpoints::github(), token, "").user()
}

/// What a repository offers for syncing.
#[derive(Debug, Clone)]
pub struct RepoInfo {
    pub repo: String,
    pub private: bool,
    /// QuickNotes data in it: the wrapped key.
    pub vault: Option<Lock>,
}

/// Checks a repository (`owner/name`) for syncing.
pub fn inspect(token: &str, repo: &str) -> Result<RepoInfo> {
    Client::new(Endpoints::github(), token, repo).inspect()
}

/// Sets up QuickNotes data in a repository: stores the wrapped key.
pub fn create_vault(token: &str, repo: &str, lock: &Lock) -> Result<()> {
    Client::new(Endpoints::github(), token, repo).create_vault(lock)
}

/// The readable file with the wrapped key.
#[derive(Serialize, Deserialize)]
struct VaultFile {
    #[serde(default)]
    about: String,
    format: u32,
    lock: Lock,
}

/// The tip of the branch.
struct Head {
    commit: String,
    tree: String,
}

struct Client {
    agent: ureq::Agent,
    token: String,
    ep: Endpoints,
    repo: String,
    branch: std::cell::RefCell<Option<String>>,
}

impl Client {
    fn new(ep: Endpoints, token: &str, repo: &str) -> Self {
        Client { agent: agent(), token: token.to_owned(), ep, repo: repo.to_owned(), branch: Default::default() }
    }

    fn request(&self, method: &str, path: &str) -> ureq::Request {
        self.agent
            .request(method, &format!("{}{path}", self.ep.api))
            .set("Authorization", &format!("Bearer {}", self.token))
            .set("Accept", "application/vnd.github+json")
            .set("X-GitHub-Api-Version", "2022-11-28")
    }

    fn get(&self, path: &str) -> Result<serde_json::Value> {
        Ok(self.request("GET", path).call()?.into_json()?)
    }

    fn send(&self, method: &str, path: &str, body: serde_json::Value) -> Result<serde_json::Value> {
        Ok(self.request(method, path).send_json(body)?.into_json()?)
    }

    fn user(&self) -> Result<String> {
        Ok(self.get("/user")?["login"].as_str().unwrap_or_default().to_owned())
    }

    fn repo_path(&self, rest: &str) -> String {
        format!("/repos/{}{rest}", self.repo)
    }

    /// The repository's metadata; sets the branch.
    fn metadata(&self) -> Result<serde_json::Value> {
        let v = self.get(&self.repo_path(""))?;
        let branch = v["default_branch"].as_str().unwrap_or("main").to_owned();
        *self.branch.borrow_mut() = Some(branch);
        Ok(v)
    }

    fn branch(&self) -> Result<String> {
        if self.branch.borrow().is_none() {
            self.metadata()?;
        }
        Ok(self.branch.borrow().clone().unwrap_or_default())
    }

    fn inspect(&self) -> Result<RepoInfo> {
        let v = self.metadata()?;
        if v["permissions"]["push"] == false {
            return Err(Error::Repo(t!("No write access to {}", self.repo)));
        }
        let repo = v["full_name"].as_str().unwrap_or(&self.repo).to_owned();
        let private = v["private"] != false;
        let branch = self.branch()?;
        let path = self.repo_path(&format!("/contents/{VAULT}?ref={}", urlencoding::encode(&branch)));
        let vault = match self.request("GET", &path).call() {
            Ok(r) => {
                let v: serde_json::Value = r.into_json()?;
                let text = decode(v["content"].as_str().unwrap_or_default())?;
                let file: VaultFile = serde_json::from_str(&text).map_err(|_| Error::Repo(t!("The QuickNotes data in the repository is damaged").into()))?;
                if file.format > FORMAT {
                    return Err(Error::Repo(t!("The repository was set up by a newer QuickNotes – please update").into()));
                }
                Some(file.lock)
            }
            // 404: no such file; 409: empty repository.
            Err(ureq::Error::Status(404 | 409, _)) => None,
            Err(e) => return Err(e.into()),
        };
        Ok(RepoInfo { repo, private, vault })
    }

    /// Writes `vault.json` with the contents API (which also works in an empty repository).
    fn create_vault(&self, lock: &Lock) -> Result<()> {
        let file = VaultFile {
            about: "Encrypted notes of QuickNotes. The key is protected by the sync password.".into(),
            format: FORMAT,
            lock: lock.clone(),
        };
        let json = serde_json::to_string_pretty(&file).map_err(|e| Error::Net(e.to_string()))?;
        self.send(
            "PUT",
            &self.repo_path(&format!("/contents/{VAULT}")),
            serde_json::json!({ "message": "Set up QuickNotes sync", "content": b64().encode(json) }),
        )?;
        Ok(())
    }

    /// The tip of the branch; `None` for an empty repository.
    fn head(&self) -> Result<Option<Head>> {
        let branch = self.branch()?;
        let commit = match self.request("GET", &self.repo_path(&format!("/git/ref/heads/{branch}"))).call() {
            Ok(r) => r.into_json::<serde_json::Value>()?["object"]["sha"].as_str().unwrap_or_default().to_owned(),
            Err(ureq::Error::Status(404 | 409, _)) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let v = self.get(&self.repo_path(&format!("/git/commits/{commit}")))?;
        let tree = v["tree"]["sha"].as_str().unwrap_or_default().to_owned();
        Ok(Some(Head { commit, tree }))
    }

    /// QuickNotes' files in a tree: path → blob id.
    fn files(&self, tree: &str) -> Result<HashMap<String, String>> {
        let v = self.get(&self.repo_path(&format!("/git/trees/{tree}?recursive=1")))?;
        if v["truncated"] == true {
            return Err(Error::Repo(t!("The repository is too large – please use a separate repository for QuickNotes").into()));
        }
        Ok(v["tree"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|e| e["type"] == "blob")
            .filter_map(|e| Some((e["path"].as_str()?.to_owned(), e["sha"].as_str()?.to_owned())))
            .filter(|(path, _)| path.starts_with(DIR))
            .collect())
    }

    fn blob(&self, sha: &str) -> Result<String> {
        let v = self.get(&self.repo_path(&format!("/git/blobs/{sha}")))?;
        decode(v["content"].as_str().unwrap_or_default())
    }

    /// One commit on top of `head` that writes (`Some`) or deletes (`None`)
    /// files. [`Error::Moved`] when the branch moved in between.
    fn commit(&self, head: &Head, writes: &[(String, Option<String>)], message: &str) -> Result<()> {
        let entries: Vec<serde_json::Value> = writes
            .iter()
            .map(|(path, content)| match content {
                Some(c) => serde_json::json!({ "path": path, "mode": "100644", "type": "blob", "content": c }),
                None => serde_json::json!({ "path": path, "mode": "100644", "type": "blob", "sha": null }),
            })
            .collect();
        let tree = self.send("POST", &self.repo_path("/git/trees"), serde_json::json!({ "base_tree": head.tree, "tree": entries }))?;
        let commit = self.send(
            "POST",
            &self.repo_path("/git/commits"),
            serde_json::json!({ "message": message, "tree": tree["sha"], "parents": [head.commit] }),
        )?;
        let branch = self.branch()?;
        match self
            .request("PATCH", &self.repo_path(&format!("/git/refs/heads/{branch}")))
            .send_json(serde_json::json!({ "sha": commit["sha"], "force": false }))
        {
            Ok(_) => Ok(()),
            // Not a fast-forward: someone else pushed.
            Err(ureq::Error::Status(409 | 422, _)) => Err(Error::Moved),
            Err(e) => Err(e.into()),
        }
    }
}

// ===== sync data ============================================================

/// Metadata of a note that is synced.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(default)]
pub struct Meta {
    /// User-defined title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// The language was chosen by hand (otherwise it follows the content).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub language_locked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub trashed: bool,
}

impl Meta {
    /// Three-way merge field by field: what changed locally wins, otherwise
    /// the remote value.
    fn merge(local: &Meta, base: &Meta, remote: &Meta) -> Meta {
        fn pick<T: Clone + PartialEq>(l: &T, b: &T, r: &T) -> T {
            if l != b { l.clone() } else { r.clone() }
        }
        Meta {
            title: pick(&local.title, &base.title, &remote.title),
            language: pick(&local.language, &base.language, &remote.language),
            language_locked: pick(&local.language_locked, &base.language_locked, &remote.language_locked),
            folder: pick(&local.folder, &base.folder, &remote.folder),
            trashed: pick(&local.trashed, &base.trashed, &remote.trashed),
        }
    }
}

/// A local note as it is synced.
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub id: String,
    pub meta: Meta,
    /// Title shown in the library while the note is closed. It goes along
    /// with the content (it is derived from it, possibly translated).
    pub label: Option<String>,
    /// Tab password: `content` is then the sealed content (Base64).
    pub lock: Option<Lock>,
    pub content: String,
}

impl Note {
    pub fn hash(&self) -> String {
        fingerprint(self.lock.as_ref(), &self.content)
    }
}

/// Stable fingerprint (FNV-1a, 64 bit) of a note's content.
pub fn fingerprint(lock: Option<&Lock>, content: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    let lock = lock.map(|l| l.key.as_str()).unwrap_or("");
    for part in [lock, "\u{0}", content] {
        for b in part.bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    format!("{h:016x}")
}

/// A folder as it is synced (without the local collapsed state).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SyncFolder {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
}

impl From<&crate::store::Folder> for SyncFolder {
    fn from(f: &crate::store::Folder) -> Self {
        SyncFolder { id: f.id.clone(), name: f.name.clone(), parent: f.parent.clone() }
    }
}

/// A note in the repository's index.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Entry {
    #[serde(flatten)]
    pub meta: Meta,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lock: Option<Lock>,
}

/// `quicknotes/index.enc`, decrypted.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Index {
    pub format: u32,
    pub folders: Vec<SyncFolder>,
    pub notes: BTreeMap<String, Entry>,
}

/// What was synced last time.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct State {
    /// Repository the state belongs to; another one starts afresh.
    pub repo: String,
    pub notes: BTreeMap<String, Base>,
    pub folders: Vec<SyncFolder>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Base {
    pub hash: String,
    pub meta: Meta,
}

const STATE_AAD: &[u8] = b"quicknotes-sync-state";

fn state_path() -> std::path::PathBuf {
    crate::store::data_dir().join("github-sync.json")
}

/// The sync state; with the app password it is stored encrypted (it holds titles).
pub fn load_state() -> State {
    let Ok(text) = std::fs::read_to_string(state_path()) else { return State::default() };
    if let Ok(state) = serde_json::from_str(&text) {
        return state;
    }
    crate::store::app_key()
        .and_then(|key| key.open_text(&text, STATE_AAD).ok())
        .and_then(|plain| serde_json::from_str(&plain).ok())
        .unwrap_or_default()
}

pub fn save_state(state: &State) {
    let Ok(json) = serde_json::to_string(state) else { return };
    let data = match crate::store::app_key() {
        Some(key) => key.seal_text(&json, STATE_AAD),
        None => json,
    };
    let path = state_path();
    let tmp = path.with_extension("tmp");
    if std::fs::write(&tmp, data).is_ok() {
        let _ = std::fs::rename(tmp, path);
    }
}

pub fn clear_state() {
    let _ = std::fs::remove_file(state_path());
}

// ===== merge ================================================================

/// How a remote note's content comes down.
#[derive(Debug, PartialEq, Eq)]
enum Fetch {
    /// Into the local note of the same id.
    Same,
    /// Into a new local note (conflict copy) with this id, which is also
    /// stored in the repository.
    Copy(String),
}

#[derive(Debug, Default)]
struct Plan {
    /// The repository's new index.
    index: Index,
    /// Local notes whose content goes up.
    upload: Vec<String>,
    /// Notes whose file is deleted in the repository.
    remove: Vec<String>,
    fetch: Vec<(String, Fetch)>,
    /// Local notes that only get new metadata.
    meta: Vec<(String, Meta)>,
    /// Local notes to delete.
    delete: Vec<String>,
    /// The new sync state.
    base: BTreeMap<String, Base>,
    folders: Vec<SyncFolder>,
    conflicts: usize,
}

/// Title of the conflict copy of a note.
fn copy_title(meta: &Meta, label: Option<&str>, stamp: &str) -> String {
    let base = meta.title.clone().or_else(|| label.map(str::to_owned)).unwrap_or_else(|| t!("Note").to_owned());
    format!("{base} ({} {stamp})", t!("conflict"))
}

/// Decides what happens to every note and folder. Notes in `skip` exist
/// locally but are left alone (still loading, unlocked), so they don't count
/// as deleted.
fn plan(
    local: &BTreeMap<String, Note>,
    skip: &HashSet<String>,
    state: &State,
    remote: &Index,
    local_folders: &[SyncFolder],
    new_id: &mut dyn FnMut() -> String,
    stamp: &str,
) -> Plan {
    let mut p = Plan { index: Index { format: FORMAT, ..Default::default() }, ..Default::default() };
    let ids: BTreeSet<&String> = local.keys().chain(state.notes.keys()).chain(remote.notes.keys()).collect();
    for id in ids {
        let (l, b, r) = (local.get(id), state.notes.get(id), remote.notes.get(id));
        if skip.contains(id) {
            if let Some(r) = r {
                p.index.notes.insert(id.clone(), r.clone());
            }
            if let Some(b) = b {
                p.base.insert(id.clone(), b.clone());
            }
            continue;
        }
        match (l, r) {
            (Some(l), Some(r)) => {
                let hash = l.hash();
                let meta = Meta::merge(&l.meta, b.map_or(&r.meta, |b| &b.meta), &r.meta);
                // Which content changed since the last sync.
                let (ours, theirs) = match b {
                    _ if hash == r.hash => (false, false),
                    Some(b) if hash == b.hash => (false, true),
                    Some(b) if r.hash == b.hash => (true, false),
                    _ => (true, true),
                };
                if theirs && !ours {
                    p.fetch.push((id.clone(), Fetch::Same));
                    let e = Entry { meta: meta.clone(), label: r.label.clone(), hash: r.hash.clone(), lock: r.lock.clone() };
                    p.index.notes.insert(id.clone(), e);
                    p.base.insert(id.clone(), Base { hash: r.hash.clone(), meta });
                    continue;
                }
                if ours {
                    p.upload.push(id.clone());
                }
                if ours && theirs {
                    // Both changed: the local version keeps the note, the
                    // remote one becomes a copy next to it.
                    let copy = new_id();
                    let copy_meta = Meta { title: Some(copy_title(&r.meta, r.label.as_deref(), stamp)), ..r.meta.clone() };
                    let e = Entry { meta: copy_meta.clone(), label: r.label.clone(), hash: r.hash.clone(), lock: r.lock.clone() };
                    p.index.notes.insert(copy.clone(), e);
                    p.base.insert(copy.clone(), Base { hash: r.hash.clone(), meta: copy_meta });
                    p.fetch.push((id.clone(), Fetch::Copy(copy)));
                    p.conflicts += 1;
                }
                if meta != l.meta {
                    p.meta.push((id.clone(), meta.clone()));
                }
                // The label changes with the content only, so different UI
                // languages don't send it back and forth.
                let label = if ours { l.label.clone() } else { r.label.clone() };
                p.index.notes.insert(id.clone(), Entry { meta: meta.clone(), label, hash: hash.clone(), lock: l.lock.clone() });
                p.base.insert(id.clone(), Base { hash, meta });
            }
            (Some(l), None) => {
                let hash = l.hash();
                if b.is_some_and(|b| b.hash == hash && b.meta == l.meta) {
                    // Deleted elsewhere, unchanged here.
                    p.delete.push(id.clone());
                } else {
                    p.upload.push(id.clone());
                    let e = Entry { meta: l.meta.clone(), label: l.label.clone(), hash: hash.clone(), lock: l.lock.clone() };
                    p.index.notes.insert(id.clone(), e);
                    p.base.insert(id.clone(), Base { hash, meta: l.meta.clone() });
                }
            }
            (None, Some(r)) => {
                if b.is_some_and(|b| b.hash == r.hash) {
                    // Deleted here, its content unchanged elsewhere.
                    p.remove.push(id.clone());
                } else {
                    p.fetch.push((id.clone(), Fetch::Same));
                    p.index.notes.insert(id.clone(), r.clone());
                    p.base.insert(id.clone(), Base { hash: r.hash.clone(), meta: r.meta.clone() });
                }
            }
            (None, None) => {}
        }
    }
    p.folders = merge_folders(local_folders, &state.folders, &remote.folders);
    p.index.folders = p.folders.clone();
    p
}

/// Three-way merge of the folders (by id, name and parent separately).
fn merge_folders(local: &[SyncFolder], base: &[SyncFolder], remote: &[SyncFolder]) -> Vec<SyncFolder> {
    let find = |list: &[SyncFolder], id: &str| list.iter().find(|f| f.id == id).cloned();
    let mut ids: Vec<&String> = local.iter().chain(remote.iter()).map(|f| &f.id).collect();
    let mut seen = HashSet::new();
    ids.retain(|id| seen.insert(*id));
    let mut out = Vec::new();
    for id in ids {
        let (l, b, r) = (find(local, id), find(base, id), find(remote, id));
        let merged = match (l, r) {
            (Some(l), Some(r)) => {
                let b = b.unwrap_or_else(|| r.clone());
                Some(SyncFolder {
                    id: id.clone(),
                    name: if l.name != b.name { l.name } else { r.name },
                    parent: if l.parent != b.parent { l.parent } else { r.parent },
                })
            }
            // Deleted on the other side: gone unless changed here.
            (Some(l), None) => (b.as_ref() != Some(&l)).then_some(l),
            (None, Some(r)) => (b.as_ref() != Some(&r)).then_some(r),
            (None, None) => None,
        };
        out.extend(merged);
    }
    // Parents that no longer exist or that would form a cycle.
    let known: HashSet<String> = out.iter().map(|f| f.id.clone()).collect();
    for i in 0..out.len() {
        if out[i].parent.as_ref().is_some_and(|p| !known.contains(p)) {
            out[i].parent = None;
        }
        let mut seen = HashSet::from([out[i].id.clone()]);
        let mut cur = out[i].parent.clone();
        while let Some(p) = cur {
            if !seen.insert(p.clone()) {
                out[i].parent = None;
                break;
            }
            cur = out.iter().find(|f| f.id == p).and_then(|f| f.parent.clone());
        }
    }
    out
}

// ===== sync =================================================================

/// A change to apply locally.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    /// Create or update a note; `content` is `None` when only the metadata changed.
    Put { id: String, meta: Meta, label: Option<String>, lock: Option<Lock>, content: Option<String> },
    Delete { id: String },
}

#[derive(Debug, Default)]
pub struct Outcome {
    pub state: State,
    pub changes: Vec<Change>,
    /// The merged folders.
    pub folders: Vec<SyncFolder>,
    /// Notes changed in the repository.
    pub sent: usize,
    pub conflicts: usize,
}

/// Everything a sync run needs from the window.
pub struct Job {
    pub repo: String,
    pub notes: Vec<Note>,
    pub skip: HashSet<String>,
    pub folders: Vec<SyncFolder>,
    pub state: State,
    /// Date and time for conflict copies.
    pub stamp: String,
}

fn note_path(id: &str) -> String {
    format!("{DIR}notes/{id}.enc")
}

/// Runs one sync (blocking); token and key come from the keyring.
pub fn sync(job: Job) -> Result<Outcome> {
    let token = secret::load(secret::TOKEN).ok_or_else(|| Error::Auth(t!("Not signed in").into()))?;
    let key = secret::load(&secret::key_name(&job.repo)).and_then(|k| Key::import(&k).ok()).ok_or(Error::Key)?;
    let client = Client::new(Endpoints::github(), &token, &job.repo);
    run(&client, &key, job, &mut crate::store::new_id)
}

fn run(gh: &Client, key: &Key, job: Job, new_id: &mut dyn FnMut() -> String) -> Result<Outcome> {
    let repo = job.repo.to_lowercase();
    let state = if job.state.repo == repo { job.state } else { State { repo, ..Default::default() } };
    let local: BTreeMap<String, Note> = job.notes.into_iter().map(|n| (n.id.clone(), n)).collect();
    let no_data = || Error::Repo(t!("The repository has no QuickNotes data – connect it again in the preferences").into());
    for _ in 0..TRIES {
        let head = gh.head()?.ok_or_else(no_data)?;
        let files = gh.files(&head.tree)?;
        if !files.contains_key(VAULT) {
            return Err(no_data());
        }
        let remote: Index = match files.get(INDEX) {
            Some(sha) => {
                let plain = key.open_text(&gh.blob(sha)?, INDEX_AAD).map_err(|_| Error::Key)?;
                let index: Index = serde_json::from_str(&plain).map_err(|_| no_data())?;
                if index.format > FORMAT {
                    return Err(Error::Repo(t!("The repository was set up by a newer QuickNotes – please update").into()));
                }
                index
            }
            None => Index::default(),
        };
        let plan = plan(&local, &job.skip, &state, &remote, &job.folders, new_id, &job.stamp);

        let mut changes = Vec::new();
        let mut writes: Vec<(String, Option<String>)> = Vec::new();
        for (id, fetch) in &plan.fetch {
            let sha = files.get(&note_path(id)).ok_or_else(no_data)?;
            let content = key.open_text(&gh.blob(sha)?, id.as_bytes()).map_err(|_| Error::Key)?;
            let target = match fetch {
                Fetch::Same => id,
                Fetch::Copy(copy) => {
                    writes.push((note_path(copy), Some(key.seal_text(&content, copy.as_bytes()))));
                    copy
                }
            };
            let e = &plan.index.notes[target];
            changes.push(Change::Put {
                id: target.clone(),
                meta: e.meta.clone(),
                label: e.label.clone(),
                lock: e.lock.clone(),
                content: Some(content.to_string()),
            });
        }
        for (id, meta) in &plan.meta {
            let l = &local[id];
            changes.push(Change::Put { id: id.clone(), meta: meta.clone(), label: l.label.clone(), lock: l.lock.clone(), content: None });
        }
        changes.extend(plan.delete.iter().map(|id| Change::Delete { id: id.clone() }));
        for id in &plan.upload {
            writes.push((note_path(id), Some(key.seal_text(&local[id].content, id.as_bytes()))));
        }
        for id in &plan.remove {
            if files.contains_key(&note_path(id)) {
                writes.push((note_path(id), None));
            }
        }
        let ids: BTreeSet<&String> = plan.index.notes.keys().chain(remote.notes.keys()).collect();
        let sent = ids.into_iter().filter(|id| plan.index.notes.get(*id) != remote.notes.get(*id)).count();
        if !writes.is_empty() || plan.index != remote {
            let json = serde_json::to_string(&plan.index).map_err(|e| Error::Net(e.to_string()))?;
            writes.push((INDEX.to_owned(), Some(key.seal_text(&json, INDEX_AAD))));
            match gh.commit(&head, &writes, "QuickNotes sync") {
                Ok(()) => {}
                Err(Error::Moved) => continue,
                Err(e) => return Err(e),
            }
        }
        let state = State { repo: state.repo.clone(), notes: plan.base, folders: plan.folders.clone() };
        return Ok(Outcome { state, changes, folders: plan.folders, sent, conflicts: plan.conflicts });
    }
    Err(Error::Moved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::Mutex;

    #[test]
    fn repository_names() {
        for input in ["me/notes", " https://github.com/me/notes ", "github.com/me/notes/", "git@github.com:me/notes.git"] {
            assert_eq!(parse_repo(input).as_deref(), Some("me/notes"), "{input}");
        }
        for input in ["", "notes", "me/", "/notes", "me/no tes", "me/..", "a/b/c"] {
            assert_eq!(parse_repo(input), None, "{input}");
        }
    }

    fn note(id: &str, content: &str) -> Note {
        Note { id: id.into(), meta: Meta::default(), label: None, lock: None, content: content.into() }
    }

    fn entry(n: &Note) -> Entry {
        Entry { meta: n.meta.clone(), label: n.label.clone(), hash: n.hash(), lock: None }
    }

    fn base(n: &Note) -> Base {
        Base { hash: n.hash(), meta: n.meta.clone() }
    }

    fn local(notes: &[&Note]) -> BTreeMap<String, Note> {
        notes.iter().map(|n| (n.id.clone(), (*n).clone())).collect()
    }

    fn index(notes: &[&Note]) -> Index {
        Index { format: FORMAT, notes: notes.iter().map(|n| (n.id.clone(), entry(n))).collect(), ..Default::default() }
    }

    fn state(notes: &[&Note]) -> State {
        State { notes: notes.iter().map(|n| (n.id.clone(), base(n))).collect(), ..Default::default() }
    }

    fn run_plan(l: &[&Note], s: &State, r: &Index) -> Plan {
        let mut n = 0;
        plan(&local(l), &HashSet::new(), s, r, &[], &mut || { n += 1; format!("copy{n}") }, "now")
    }

    #[test]
    fn new_notes_go_both_ways() {
        let (a, b) = (note("a", "x"), note("b", "y"));
        let p = run_plan(&[&a], &State::default(), &index(&[&b]));
        assert_eq!(p.upload, ["a"]);
        assert_eq!(p.fetch, [("b".to_owned(), Fetch::Same)]);
        assert_eq!(p.index.notes.len(), 2);
        assert_eq!(p.base.len(), 2);
    }

    #[test]
    fn changes_flow_in_the_right_direction() {
        let old = note("a", "old");
        let new = note("a", "new");
        // Changed here.
        let p = run_plan(&[&new], &state(&[&old]), &index(&[&old]));
        assert_eq!((p.upload.as_slice(), p.fetch.len()), (["a".to_owned()].as_slice(), 0));
        assert_eq!(p.index.notes["a"].hash, new.hash());
        // Changed there.
        let p = run_plan(&[&old], &state(&[&old]), &index(&[&new]));
        assert_eq!((p.upload.len(), p.fetch.as_slice()), (0, [("a".to_owned(), Fetch::Same)].as_slice()));
        assert_eq!(p.base["a"].hash, new.hash());
        // Unchanged.
        let p = run_plan(&[&old], &state(&[&old]), &index(&[&old]));
        assert!(p.upload.is_empty() && p.fetch.is_empty() && p.meta.is_empty());
        assert_eq!(p.index, index(&[&old]));
    }

    #[test]
    fn metadata_merges_field_by_field() {
        let synced = Note { meta: Meta { title: Some("T".into()), folder: Some("f1".into()), ..Default::default() }, ..note("a", "x") };
        let mut here = synced.clone();
        here.meta.title = Some("Renamed".into());
        let mut there = synced.clone();
        there.meta.folder = Some("f2".into());
        let p = run_plan(&[&here], &state(&[&synced]), &index(&[&there]));
        let merged = Meta { title: Some("Renamed".into()), folder: Some("f2".into()), ..Default::default() };
        assert_eq!(p.index.notes["a"].meta, merged);
        assert_eq!(p.meta, [("a".to_owned(), merged)]);
        assert!(p.upload.is_empty() && p.fetch.is_empty(), "content unchanged");
    }

    #[test]
    fn labels_go_with_the_content() {
        let synced = Note { label: Some("Untitled".into()), ..note("a", "") };
        // Same content, label in another language: nothing to send.
        let german = Note { label: Some("Unbenannt".into()), ..synced.clone() };
        let p = run_plan(&[&german], &state(&[&synced]), &index(&[&synced]));
        assert_eq!(p.index, index(&[&synced]));
        assert!(p.meta.is_empty());
        // New content: its label goes along.
        let edited = Note { label: Some("Hello".into()), ..note("a", "Hello") };
        let p = run_plan(&[&edited], &state(&[&synced]), &index(&[&synced]));
        assert_eq!(p.index.notes["a"].label.as_deref(), Some("Hello"));
    }

    #[test]
    fn conflicts_keep_both_versions() {
        let old = note("a", "old");
        let (ours, theirs) = (note("a", "ours"), note("a", "theirs"));
        let p = run_plan(&[&ours], &state(&[&old]), &index(&[&theirs]));
        assert_eq!(p.conflicts, 1);
        assert_eq!(p.upload, ["a"]);
        assert_eq!(p.fetch, [("a".to_owned(), Fetch::Copy("copy1".into()))]);
        assert_eq!(p.index.notes["a"].hash, ours.hash());
        assert_eq!(p.index.notes["copy1"].hash, theirs.hash());
        assert_eq!(p.index.notes["copy1"].meta.title.as_deref(), Some("Note (conflict now)"));
        // Never synced before, different content: the same.
        let p = run_plan(&[&ours], &State::default(), &index(&[&theirs]));
        assert_eq!(p.conflicts, 1);
        // Never synced before, same content: just linked.
        let p = run_plan(&[&ours], &State::default(), &index(&[&ours]));
        assert!(p.upload.is_empty() && p.fetch.is_empty() && p.conflicts == 0);
        assert_eq!(p.base["a"].hash, ours.hash());
    }

    #[test]
    fn deletions() {
        let a = note("a", "x");
        let changed = note("a", "y");
        // Deleted there, unchanged here: deleted here.
        let p = run_plan(&[&a], &state(&[&a]), &Index::default());
        assert_eq!(p.delete, ["a"]);
        assert!(p.index.notes.is_empty() && p.base.is_empty());
        // Deleted there, changed here: kept and sent again.
        let p = run_plan(&[&changed], &state(&[&a]), &Index::default());
        assert_eq!(p.upload, ["a"]);
        // Deleted here, unchanged there: removed there.
        let p = run_plan(&[], &state(&[&a]), &index(&[&a]));
        assert_eq!(p.remove, ["a"]);
        assert!(p.index.notes.is_empty());
        // Deleted here, changed there: comes back.
        let p = run_plan(&[], &state(&[&a]), &index(&[&changed]));
        assert_eq!(p.fetch, [("a".to_owned(), Fetch::Same)]);
        // Gone everywhere: forgotten.
        let p = run_plan(&[], &state(&[&a]), &Index::default());
        assert!(p.base.is_empty() && p.remove.is_empty() && p.delete.is_empty());
    }

    #[test]
    fn skipped_notes_are_left_alone() {
        let a = note("a", "x");
        let p = plan(&local(&[]), &HashSet::from(["a".to_owned()]), &state(&[&a]), &index(&[&a]), &[], &mut String::new, "");
        assert!(p.remove.is_empty() && p.delete.is_empty());
        assert_eq!(p.index, index(&[&a]));
        assert_eq!(p.base["a"], base(&a));
    }

    fn folder(id: &str, name: &str, parent: Option<&str>) -> SyncFolder {
        SyncFolder { id: id.into(), name: name.into(), parent: parent.map(Into::into) }
    }

    #[test]
    fn folders_merge_three_way() {
        let base = [folder("a", "A", None), folder("b", "B", None), folder("c", "C", None)];
        // Here: a renamed, c deleted, d new. There: a moved into b, b deleted, e new.
        let local = [folder("a", "A2", None), folder("b", "B", None), folder("d", "D", None)];
        let remote = [folder("a", "A", Some("b")), folder("c", "C", None), folder("e", "E", Some("c"))];
        let merged = merge_folders(&local, &base, &remote);
        // b is gone (deleted there, unchanged here), so a lands at the top;
        // c is gone (deleted here), so e does too.
        assert_eq!(merged, [folder("a", "A2", None), folder("d", "D", None), folder("e", "E", None)]);
    }

    #[test]
    fn folder_cycles_are_broken() {
        let base = [folder("a", "A", None), folder("b", "B", None)];
        let local = [folder("a", "A", Some("b")), folder("b", "B", None)];
        let remote = [folder("a", "A", None), folder("b", "B", Some("a"))];
        let merged = merge_folders(&local, &base, &remote);
        assert_eq!(merged, [folder("a", "A", None), folder("b", "B", Some("a"))]);
    }

    // ----- end-to-end against a mock of the GitHub API ---------------------

    #[derive(Default)]
    struct Repo {
        /// Branch tip.
        head: Option<String>,
        /// Commit → (tree, parent).
        commits: HashMap<String, (String, Option<String>)>,
        /// Tree → path → blob.
        trees: HashMap<String, BTreeMap<String, String>>,
        blobs: HashMap<String, String>,
        next: usize,
        /// Pushes from "another machine" to simulate before the next ref update.
        races: usize,
        polls: usize,
    }

    impl Repo {
        fn id(&mut self) -> String {
            self.next += 1;
            format!("{:040x}", self.next)
        }

        fn files(&self) -> BTreeMap<String, String> {
            self.head
                .as_ref()
                .map(|c| self.trees[&self.commits[c].0].iter().map(|(p, b)| (p.clone(), self.blobs[b].clone())).collect())
                .unwrap_or_default()
        }

        /// A commit with `files` on top of the head.
        fn commit_files(&mut self, files: BTreeMap<String, String>) -> String {
            let mut tree = BTreeMap::new();
            for (path, content) in files {
                let blob = self.id();
                self.blobs.insert(blob.clone(), content);
                tree.insert(path, blob);
            }
            let tree_id = self.id();
            self.trees.insert(tree_id.clone(), tree);
            let commit = self.id();
            self.commits.insert(commit.clone(), (tree_id, self.head.clone()));
            commit
        }
    }

    type Shared = Arc<Mutex<Repo>>;

    fn read_request(stream: &mut std::net::TcpStream) -> Option<(String, String, Vec<u8>)> {
        let mut buf = Vec::new();
        let mut byte = [0u8; 1];
        while !buf.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).ok()?;
            buf.push(byte[0]);
        }
        let head = String::from_utf8_lossy(&buf).to_string();
        let mut first = head.lines().next()?.split_whitespace();
        let (method, target) = (first.next()?.to_owned(), first.next()?.to_owned());
        let len = head
            .lines()
            .find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse().unwrap()))
            .unwrap_or(0);
        let mut body = vec![0u8; len];
        stream.read_exact(&mut body).ok()?;
        Some((method, target, body))
    }

    /// A tiny HTTP server implementing the calls used here, for `me/notes`.
    fn mock_github() -> (Endpoints, Shared) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        let shared: Shared = Default::default();
        let repo = shared.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let Some((method, target, body)) = read_request(&mut stream) else { continue };
                let path = target.split('?').next().unwrap_or("").to_owned();
                let body: serde_json::Value = if method == "POST" && path.starts_with("/login") {
                    serde_json::Value::Null
                } else {
                    serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null)
                };
                let mut r = repo.lock().unwrap();
                let rest = path.strip_prefix("/repos/me/notes").map(str::to_owned);
                let (status, reply): (u16, serde_json::Value) = match (method.as_str(), path.as_str(), rest.as_deref()) {
                    ("POST", "/login/device/code", _) => (
                        200,
                        serde_json::json!({ "device_code": "dc", "user_code": "ABCD-1234",
                            "verification_uri": "https://github.com/login/device", "expires_in": 60, "interval": 0 }),
                    ),
                    ("POST", "/login/oauth/access_token", _) => {
                        r.polls += 1;
                        if r.polls < 3 {
                            (200, serde_json::json!({ "error": "authorization_pending" }))
                        } else {
                            (200, serde_json::json!({ "access_token": "tok", "token_type": "bearer" }))
                        }
                    }
                    ("GET", "/user", _) => (200, serde_json::json!({ "login": "me" })),
                    ("GET", _, Some("")) => (
                        200,
                        serde_json::json!({ "full_name": "me/notes", "private": false, "default_branch": "main",
                            "permissions": { "push": true } }),
                    ),
                    ("GET", _, Some(p)) if p.starts_with("/contents/") => {
                        match r.files().get(&p["/contents/".len()..]) {
                            Some(c) => (200, serde_json::json!({ "content": b64().encode(c), "encoding": "base64" })),
                            None if r.head.is_none() => (404, serde_json::json!({ "message": "This repository is empty." })),
                            None => (404, serde_json::json!({ "message": "Not Found" })),
                        }
                    }
                    ("PUT", _, Some(p)) if p.starts_with("/contents/") => {
                        let mut files = r.files();
                        files.insert(p["/contents/".len()..].to_owned(), decode(body["content"].as_str().unwrap()).unwrap());
                        let c = r.commit_files(files);
                        r.head = Some(c);
                        (201, serde_json::json!({}))
                    }
                    ("GET", _, Some("/git/ref/heads/main")) => match &r.head {
                        Some(c) => (200, serde_json::json!({ "object": { "sha": c } })),
                        None => (409, serde_json::json!({ "message": "Git Repository is empty." })),
                    },
                    ("GET", _, Some(p)) if p.starts_with("/git/commits/") => {
                        let (tree, _) = &r.commits[&p["/git/commits/".len()..]];
                        (200, serde_json::json!({ "tree": { "sha": tree } }))
                    }
                    ("GET", _, Some(p)) if p.starts_with("/git/trees/") => {
                        let tree = &r.trees[&p["/git/trees/".len()..]];
                        let list: Vec<_> =
                            tree.iter().map(|(path, sha)| serde_json::json!({ "path": path, "type": "blob", "sha": sha })).collect();
                        (200, serde_json::json!({ "tree": list, "truncated": false }))
                    }
                    ("GET", _, Some(p)) if p.starts_with("/git/blobs/") => {
                        // GitHub wraps Base64 in lines of 60 characters.
                        let encoded = b64().encode(&r.blobs[&p["/git/blobs/".len()..]]);
                        let wrapped: Vec<String> =
                            encoded.as_bytes().chunks(60).map(|c| String::from_utf8_lossy(c).into_owned()).collect();
                        (200, serde_json::json!({ "content": wrapped.join("\n"), "encoding": "base64" }))
                    }
                    ("POST", _, Some("/git/trees")) => {
                        let mut tree = r.trees[body["base_tree"].as_str().unwrap()].clone();
                        for e in body["tree"].as_array().unwrap() {
                            let path = e["path"].as_str().unwrap().to_owned();
                            match e["content"].as_str() {
                                Some(c) => {
                                    let blob = r.id();
                                    r.blobs.insert(blob.clone(), c.to_owned());
                                    tree.insert(path, blob);
                                }
                                None => {
                                    assert!(tree.remove(&path).is_some(), "deleting a missing file: {path}");
                                }
                            }
                        }
                        let id = r.id();
                        r.trees.insert(id.clone(), tree);
                        (201, serde_json::json!({ "sha": id }))
                    }
                    ("POST", _, Some("/git/commits")) => {
                        let id = r.id();
                        let parent = body["parents"][0].as_str().map(str::to_owned);
                        r.commits.insert(id.clone(), (body["tree"].as_str().unwrap().to_owned(), parent));
                        (201, serde_json::json!({ "sha": id }))
                    }
                    ("PATCH", _, Some("/git/refs/heads/main")) => {
                        if r.races > 0 {
                            // Another machine pushes first.
                            r.races -= 1;
                            let files = r.files();
                            let c = r.commit_files(files);
                            r.head = Some(c);
                        }
                        let sha = body["sha"].as_str().unwrap().to_owned();
                        if r.commits[&sha].1 == r.head {
                            r.head = Some(sha);
                            (200, serde_json::json!({}))
                        } else {
                            (422, serde_json::json!({ "message": "Update is not a fast forward" }))
                        }
                    }
                    _ => (404, serde_json::json!({ "message": "Not Found" })),
                };
                drop(r);
                let reply = reply.to_string();
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                    reply.len()
                );
            }
        });
        (Endpoints { api: base.clone(), login: base }, shared)
    }

    /// One machine: its notes, folders and sync state.
    struct Machine {
        notes: Vec<Note>,
        folders: Vec<SyncFolder>,
        state: State,
        ids: usize,
        name: &'static str,
    }

    impl Machine {
        fn new(name: &'static str, notes: Vec<Note>) -> Self {
            Machine { notes, folders: Vec::new(), state: State::default(), ids: 0, name }
        }

        fn sync(&mut self, gh: &Client, key: &Key) -> Outcome {
            let job = Job {
                repo: "me/notes".into(),
                notes: self.notes.clone(),
                skip: HashSet::new(),
                folders: self.folders.clone(),
                state: self.state.clone(),
                stamp: "now".into(),
            };
            let name = self.name;
            let mut ids = self.ids;
            let out = run(gh, key, job, &mut || {
                ids += 1;
                format!("{name}-{ids}")
            })
            .unwrap();
            self.ids = ids;
            // Apply like the window does.
            for c in &out.changes {
                match c {
                    Change::Put { id, meta, label, lock, content } => match self.notes.iter_mut().find(|n| &n.id == id) {
                        Some(n) => {
                            n.meta = meta.clone();
                            n.label = label.clone();
                            n.lock = lock.clone();
                            if let Some(c) = content {
                                n.content = c.clone();
                            }
                        }
                        None => self.notes.push(Note {
                            id: id.clone(),
                            meta: meta.clone(),
                            label: label.clone(),
                            lock: lock.clone(),
                            content: content.clone().unwrap(),
                        }),
                    },
                    Change::Delete { id } => self.notes.retain(|n| &n.id != id),
                }
            }
            self.folders = out.folders.clone();
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
    fn device_sign_in() {
        let (ep, _) = mock_github();
        let code = device_start_at(&ep, "client").unwrap();
        assert_eq!(code.user_code, "ABCD-1234");
        let token = device_finish_at(&ep, "client", &code, Arc::new(AtomicBool::new(false))).unwrap();
        assert_eq!(token, "tok");
        assert_eq!(Client::new(ep.clone(), &token, "").user().unwrap(), "me");
        let cancelled = device_finish_at(&ep, "client", &DeviceCode { interval: 1, ..code }, Arc::new(AtomicBool::new(true)));
        assert!(matches!(cancelled, Err(Error::Cancelled)));
    }

    #[test]
    fn two_machines_stay_in_sync() {
        let (ep, repo) = mock_github();
        let gh = Client::new(ep, "tok", "me/notes");

        // An empty repository gets set up.
        let info = gh.inspect().unwrap();
        assert!(info.vault.is_none() && !info.private);
        let (lock, key) = Lock::create("sync password", crate::crypto::tests::fast_kdf());
        gh.create_vault(&lock).unwrap();
        let info = gh.inspect().unwrap();
        let key_b = info.vault.unwrap().open("sync password").unwrap();

        let mut ideas = note("n2", "# x");
        ideas.meta = Meta { title: Some("Ideas".into()), language: Some("markdown".into()), folder: Some("f".into()), ..Default::default() };
        let mut a = Machine::new("a", vec![note("n1", "first"), ideas.clone()]);
        a.folders = vec![folder("f", "Work", None)];
        let mut b = Machine::new("b", vec![]);

        // A sends both notes; B receives them with folder and metadata.
        assert_eq!(a.sync(&gh, &key).sent, 2);
        assert_eq!(a.sync(&gh, &key).sent, 0, "nothing changed");
        let commits = repo.lock().unwrap().commits.len();
        a.sync(&gh, &key);
        assert_eq!(repo.lock().unwrap().commits.len(), commits, "no empty commits");
        assert_eq!(b.sync(&gh, &key_b).changes.len(), 2);
        assert_eq!(b.notes.iter().find(|n| n.id == "n2").unwrap(), &ideas);
        assert_eq!(b.folders, [folder("f", "Work", None)]);

        // Nothing readable ends up in the repository.
        for (path, content) in repo.lock().unwrap().files() {
            assert!(!content.contains("first") && !content.contains("Ideas") && !content.contains("Work"), "{path}");
        }

        // B edits, A receives the edit.
        b.edit("n1", "first, edited on B");
        assert_eq!(b.sync(&gh, &key_b).sent, 1);
        a.sync(&gh, &key);
        assert_eq!(a.content("n1"), Some("first, edited on B"));

        // Both edit n1: both machines end up with both versions.
        a.edit("n1", "A's version");
        b.edit("n1", "B's version");
        b.sync(&gh, &key_b);
        let out = a.sync(&gh, &key);
        assert_eq!(out.conflicts, 1);
        assert_eq!(a.content("n1"), Some("A's version"));
        let copy = a.notes.iter().find(|n| n.content == "B's version").unwrap().clone();
        assert!(copy.meta.title.as_deref().unwrap().contains("conflict"));
        b.sync(&gh, &key_b);
        assert_eq!(b.content("n1"), Some("A's version"));
        assert_eq!(b.content(&copy.id), Some("B's version"));

        // Deleting on A deletes on B.
        a.notes.retain(|n| n.id != "n2");
        a.sync(&gh, &key);
        assert!(!repo.lock().unwrap().files().contains_key("quicknotes/notes/n2.enc"));
        let out = b.sync(&gh, &key_b);
        assert_eq!(out.changes, [Change::Delete { id: "n2".into() }]);

        // Another machine pushes at the same moment: the sync starts over.
        a.edit("n1", "after a race");
        repo.lock().unwrap().races = 2;
        a.sync(&gh, &key);
        b.sync(&gh, &key_b);
        assert_eq!(b.content("n1"), Some("after a race"));

        // A wrong key is reported as such.
        let wrong = run(&gh, &Key::random(), Job {
            repo: "me/notes".into(),
            notes: vec![],
            skip: HashSet::new(),
            folders: vec![],
            state: State::default(),
            stamp: String::new(),
        }, &mut String::new);
        assert!(matches!(wrong, Err(Error::Key)));
    }

    #[test]
    fn repositories_without_quicknotes_data_are_refused() {
        let (ep, repo) = mock_github();
        let gh = Client::new(ep, "tok", "me/notes");
        let job = || Job {
            repo: "me/notes".into(),
            notes: vec![note("a", "x")],
            skip: HashSet::new(),
            folders: vec![],
            state: State::default(),
            stamp: String::new(),
        };
        assert!(matches!(run(&gh, &Key::random(), job(), &mut String::new), Err(Error::Repo(_))));
        let mut r = repo.lock().unwrap();
        let c = r.commit_files(BTreeMap::from([("README.md".to_owned(), "hello".to_owned())]));
        r.head = Some(c);
        drop(r);
        assert!(matches!(run(&gh, &Key::random(), job(), &mut String::new), Err(Error::Repo(_))));
    }
}
