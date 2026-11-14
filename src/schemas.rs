//! Schema files for JSON, YAML and XML documents: local files and downloads.
//!
//! Downloads are kept in the cache directory and used from there; after a
//! week they are fetched again in the background. The SchemaStore catalog
//! maps file names (`package.json`, `docker-compose.yml`, …) to schemas.
//! All of this runs on the main thread; downloads run in worker threads and
//! report back through the listener.

use gtk::{gio, glib};
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

pub const CATALOG: &str = "https://www.schemastore.org/api/json/catalog.json";
const MAX_AGE: i64 = 7 * 24 * 3600;
const MAX_SIZE: u64 = 16 * 1024 * 1024;
/// Seconds before a failed download is tried again.
const RETRY: i64 = 300;

#[derive(Serialize, Deserialize, Default)]
struct Index {
    files: HashMap<String, Cached>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Cached {
    file: String,
    fetched: i64,
}

struct Pattern {
    re: regex::Regex,
    negated: bool,
}

struct Entry {
    url: String,
    patterns: Vec<Pattern>,
}

#[derive(Default)]
struct State {
    index: Option<Index>,
    loading: HashSet<String>,
    /// Failed downloads (retried after a few minutes) and unreadable files.
    failed: HashMap<String, (String, i64)>,
    /// When each download was last started.
    attempted: HashMap<String, i64>,
    /// Parsed JSON per URL (or path) with the modification time it was read at.
    json: HashMap<String, (&'static Json, Option<SystemTime>)>,
    text: HashMap<String, (Rc<str>, Option<SystemTime>)>,
    catalog: Option<Rc<Vec<Entry>>>,
    matches: HashMap<String, Option<String>>,
    models: HashMap<String, Rc<crate::xsd::Model>>,
    listener: Option<Rc<dyn Fn()>>,
    /// Downloaded XSD files for libxml2's entity loader.
    xsd_files: HashMap<String, PathBuf>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

fn cache_dir() -> PathBuf {
    dirs::cache_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".cache")).join("quicknotes").join("schemas")
}

fn with<T>(f: impl FnOnce(&mut State) -> T) -> T {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if s.index.is_none() {
            s.index = Some(std::fs::read(cache_dir().join("index.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default());
        }
        f(&mut s)
    })
}

/// Called (on the main thread) whenever a download finished.
pub fn set_listener(f: impl Fn() + 'static) {
    STATE.with(|s| s.borrow_mut().listener = Some(Rc::new(f)));
}

/// The file a `file://` URL or an absolute path points to.
pub fn local_path(url: &str) -> Option<PathBuf> {
    if let Some(rest) = url.strip_prefix("file://") {
        let decoded = urlencoding::decode(rest).map(|c| c.into_owned()).unwrap_or_else(|_| rest.to_owned());
        return Some(PathBuf::from(decoded));
    }
    url.starts_with('/').then(|| PathBuf::from(url))
}

fn is_remote(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://")
}

/// The `file://` URL of a path.
pub fn file_url(path: &Path) -> String {
    let encoded: Vec<String> = path.to_string_lossy().split('/').map(|s| urlencoding::encode(s).into_owned()).collect();
    format!("file://{}", encoded.join("/"))
}

/// Why a schema could not be loaded, if it failed.
pub fn failure(url: &str) -> Option<String> {
    with(|s| s.failed.get(url).map(|(e, _)| e.clone()))
}

/// A download is running for the URL.
pub fn is_loading(url: &str) -> bool {
    with(|s| s.loading.contains(url))
}

fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Starts a download when the URL is not cached (or the copy is old).
fn ensure(url: &str) {
    let start = with(|s| {
        if s.loading.contains(url) || s.attempted.get(url).is_some_and(|at| now() - at < RETRY) {
            return false;
        }
        match s.index.as_ref().and_then(|i| i.files.get(url)) {
            Some(c) => now() - c.fetched > MAX_AGE,
            None => true,
        }
    });
    if start {
        download(url.to_owned());
    }
}

fn download(url: String) {
    with(|s| {
        s.loading.insert(url.clone());
        s.attempted.insert(url.clone(), now());
    });
    let u = url.clone();
    glib::spawn_future_local(async move {
        let result = gio::spawn_blocking(move || fetch(&u)).await.unwrap_or_else(|_| Err("download failed".into()));
        let listener = with(|s| {
            s.loading.remove(&url);
            match result {
                Ok(bytes) => {
                    let name = format!("{:x}", Sha256::digest(url.as_bytes()));
                    let dir = cache_dir();
                    let _ = std::fs::create_dir_all(&dir);
                    let file = name[..32].to_owned();
                    if std::fs::write(dir.join(&file), &bytes).is_ok() {
                        let index = s.index.get_or_insert_with(Index::default);
                        index.files.insert(url.clone(), Cached { file, fetched: now() });
                        if let Ok(json) = serde_json::to_vec(index) {
                            let _ = std::fs::write(dir.join("index.json"), json);
                        }
                    }
                    s.failed.remove(&url);
                }
                Err(e) => {
                    // A cached copy stays usable when refreshing fails.
                    let cached = s.index.as_ref().is_some_and(|i| i.files.contains_key(&url));
                    if !cached {
                        s.failed.insert(url.clone(), (e, now()));
                    }
                }
            }
            s.json.remove(&url);
            s.text.remove(&url);
            s.models.clear();
            if url == CATALOG {
                s.catalog = None;
                s.matches.clear();
            }
            s.listener.clone()
        });
        if let Some(f) = listener {
            f();
        }
    });
}

fn fetch(url: &str) -> Result<Vec<u8>, String> {
    let response = ureq::get(url).timeout(std::time::Duration::from_secs(20)).call().map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    response.into_reader().take(MAX_SIZE).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes)
}

/// The text of a schema file, or `None` while it is being downloaded or
/// when it is not available (see [`failure`]).
pub fn text(url: &str) -> Option<Rc<str>> {
    let path = match local_path(url) {
        Some(p) => p,
        None if is_remote(url) => {
            ensure(url);
            with(|s| s.index.as_ref().and_then(|i| i.files.get(url)).map(|c| cache_dir().join(&c.file)))?
        }
        None => return None,
    };
    let stamp = mtime(&path);
    if let Some(found) = with(|s| s.text.get(url).filter(|(_, t)| *t == stamp).map(|(text, _)| text.clone())) {
        return Some(found);
    }
    match std::fs::read(&path) {
        Ok(bytes) => {
            let text: Rc<str> = String::from_utf8_lossy(&bytes).into();
            with(|s| {
                s.failed.remove(url);
                s.text.insert(url.to_owned(), (text.clone(), stamp));
            });
            Some(text)
        }
        Err(e) => {
            with(|s| s.failed.insert(url.to_owned(), (e.to_string(), now())));
            None
        }
    }
}

/// A JSON schema document; parsed once per version (and kept for the rest of
/// the program, see `json_schema.rs`).
pub fn json(url: &str) -> Option<&'static Json> {
    let url = crate::json_schema::strip_fragment(url);
    let stamp = local_path(url).and_then(|p| mtime(&p));
    if let Some(found) = with(|s| s.json.get(url).filter(|(_, t)| *t == stamp).map(|(j, _)| *j)) {
        return Some(found);
    }
    let text = text(url)?;
    // YAML schemas exist, too.
    let parsed: Option<Json> = serde_json::from_str(&text).ok().or_else(|| serde_yaml::from_str(&text).ok());
    match parsed {
        Some(value) => {
            let leaked: &'static Json = Box::leak(Box::new(value));
            with(|s| s.json.insert(url.to_owned(), (leaked, stamp)));
            Some(leaked)
        }
        None => {
            with(|s| s.failed.insert(url.to_owned(), (t!("not a valid JSON schema").into(), now())));
            None
        }
    }
}

/// Loads schema documents for the validator.
pub struct Loader;

impl crate::json_schema::Source for Loader {
    fn load(&self, url: &str) -> Option<&'static Json> {
        json(url)
    }
}

// ----- SchemaStore catalog ---------------------------------------------------

/// A glob of the catalog as a regular expression (`**/` any folders, `*` any
/// name part). Patterns with a `/` match the end of the path, others the file name.
fn glob(pattern: &str) -> Option<regex::Regex> {
    let p = pattern.trim_start_matches("./");
    let mut re = String::from("(?:^|/)");
    let mut chars = p.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' if chars.peek() == Some(&'*') => {
                chars.next();
                if chars.peek() == Some(&'/') {
                    chars.next();
                    re.push_str("(?:.*/)?");
                } else {
                    re.push_str(".*");
                }
            }
            '*' => re.push_str("[^/]*"),
            '?' => re.push_str("[^/]"),
            '{' => re.push_str("(?:"),
            '}' => re.push(')'),
            ',' => re.push('|'),
            c => re.push_str(&regex::escape(&c.to_string())),
        }
    }
    re.push('$');
    regex::RegexBuilder::new(&re).case_insensitive(true).build().ok()
}

fn catalog() -> Option<Rc<Vec<Entry>>> {
    if let Some(c) = with(|s| s.catalog.clone()) {
        return Some(c);
    }
    let doc = json(CATALOG)?;
    let entries: Vec<Entry> = doc
        .get("schemas")
        .and_then(Json::as_array)
        .into_iter()
        .flatten()
        .filter_map(|e| {
            let url = e.get("url")?.as_str()?.to_owned();
            let patterns: Vec<Pattern> = e
                .get("fileMatch")?
                .as_array()?
                .iter()
                .filter_map(Json::as_str)
                .filter_map(|p| {
                    let (negated, p) = match p.strip_prefix('!') {
                        Some(rest) => (true, rest),
                        None => (false, p),
                    };
                    glob(p).map(|re| Pattern { re, negated })
                })
                .collect();
            (!patterns.is_empty()).then_some(Entry { url, patterns })
        })
        .collect();
    let entries = Rc::new(entries);
    with(|s| s.catalog = Some(entries.clone()));
    Some(entries)
}

/// The catalog's schema for a file (its path, or just a name).
pub fn catalog_match(path: &str) -> Option<String> {
    if let Some(found) = with(|s| s.matches.get(path).cloned()) {
        return found;
    }
    let entries = catalog()?;
    let found = entries
        .iter()
        .find(|e| e.patterns.iter().any(|p| !p.negated && p.re.is_match(path)) && !e.patterns.iter().any(|p| p.negated && p.re.is_match(path)))
        .map(|e| e.url.clone());
    with(|s| s.matches.insert(path.to_owned(), found.clone()));
    found
}

// ----- XML Schemas -------------------------------------------------------------

/// State of the schema files (with their includes and imports) of a document.
pub enum XsdState {
    Ready,
    Loading,
    Failed(String, String),
}

/// All schema files reachable from `urls`, in order; stops at the first
/// missing one.
fn xsd_closure(urls: &[String]) -> Result<Vec<(String, Rc<str>)>, XsdState> {
    let mut todo: Vec<String> = urls.to_vec();
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    while let Some(url) = todo.pop() {
        if !seen.insert(url.clone()) || seen.len() > 200 {
            continue;
        }
        match text(&url) {
            Some(t) => {
                todo.extend(crate::xsd::dependencies(&t, &url));
                out.push((url, t));
            }
            None => {
                return Err(match failure(&url) {
                    Some(e) => XsdState::Failed(url, e),
                    None if is_remote(&url) => XsdState::Loading,
                    None => XsdState::Failed(url, t!("file not found").into()),
                });
            }
        }
    }
    Ok(out)
}

/// Makes sure the schema files are there for libxml2.
pub fn xsd_state(urls: &[String]) -> XsdState {
    match xsd_closure(urls) {
        Ok(files) => {
            let remote: HashMap<String, PathBuf> = with(|s| {
                files
                    .iter()
                    .filter(|(u, _)| is_remote(u))
                    .filter_map(|(u, _)| s.index.as_ref()?.files.get(u).map(|c| (u.clone(), cache_dir().join(&c.file))))
                    .collect()
            });
            let changed = with(|s| {
                let mut merged = s.xsd_files.clone();
                merged.extend(remote);
                let changed = merged != s.xsd_files;
                s.xsd_files = merged;
                changed.then(|| s.xsd_files.clone())
            });
            if let Some(all) = changed {
                crate::xml_lint::set_local_files(all);
            }
            XsdState::Ready
        }
        Err(state) => state,
    }
}

/// The completion model of a set of schema files (cached until a download finishes).
pub fn xsd_model(urls: &[String]) -> Option<Rc<crate::xsd::Model>> {
    let key = urls.join("\n");
    if let Some(m) = with(|s| s.models.get(&key).cloned()) {
        return Some(m);
    }
    let files = xsd_closure(urls).ok()?;
    let texts: Vec<String> = files.into_iter().map(|(_, t)| t.to_string()).collect();
    let model = Rc::new(crate::xsd::Model::new(&texts));
    with(|s| s.models.insert(key, model.clone()));
    Some(model)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs() {
        let m = |p: &str, path: &str| glob(p).unwrap().is_match(path);
        assert!(m("package.json", "/home/u/app/package.json"));
        assert!(m("package.json", "package.json"));
        assert!(!m("package.json", "/home/u/app/package.json.bak"));
        assert!(m("*.babelrc", "x.babelrc"));
        assert!(m("**/.github/workflows/*.yml", "/r/.github/workflows/ci.yml"));
        assert!(m("**/.github/workflows/*.yml", ".github/workflows/ci.yml"));
        assert!(m("docker-compose.{yml,yaml}", "docker-compose.yaml"));
        assert!(!m("*.yml", "a/b.yaml"));
    }

    #[test]
    fn urls_and_paths() {
        assert_eq!(local_path("file:///home/u/a%20b.json"), Some(PathBuf::from("/home/u/a b.json")));
        assert_eq!(file_url(Path::new("/home/u/a b.json")), "file:///home/u/a%20b.json");
        assert_eq!(local_path("https://x"), None);
    }
}
